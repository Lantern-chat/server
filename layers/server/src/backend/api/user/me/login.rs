use std::{net::SocketAddr, time::SystemTime};

use rand::RngCore;
use schema::{auth::RawAuthToken, Snowflake};

use crate::backend::{
    api::auth::AuthTokenExt,
    util::encrypt::{decrypt_user_message, encrypt_user_message},
    util::validation::validate_email,
};
use crate::{Error, ServerState};

use sdk::{
    api::commands::user::UserLoginForm,
    models::{ElevationLevel, Session, UserFlags},
};

pub async fn login(state: ServerState, addr: SocketAddr, form: UserLoginForm) -> Result<Session, Error> {
    if form.password.len() < 8 {
        return Err(Error::InvalidCredentials);
    }

    // early validation
    if let Some(ref token) = form.totp {
        validate_2fa_token(token)?;
    }

    validate_email(&form.email)?; // NOTE: Uses a regex, so it goes last.

    let db = state.db.read.get().await?;

    #[rustfmt::skip]
    let user = db.query_opt2(schema::sql! {
        SELECT
            Users.Id        AS @Id,
            Users.Flags     AS @Flags,
            Users.Passhash  AS @Passhash,
            Users.MfaSecret AS @MfaSecret,
            Users.MfaBackup AS @MfaBackup
        FROM  LiveUsers AS Users
        WHERE Users.Email = #{&form.email as Users::Email}
    }).await?;

    let Some(user) = user else { return Err(Error::InvalidCredentials); };

    let user_id: Snowflake = user.id()?;
    let flags = UserFlags::from_bits_truncate(user.flags()?);
    let passhash: &str = user.passhash()?;
    let secret: Option<&[u8]> = user.mfa_secret()?;
    let backup: Option<&[u8]> = user.mfa_backup()?;

    match flags.elevation() {
        // System user flat out cannot log in. Pretend it doesn't exist.
        ElevationLevel::System => return Err(Error::NotFound),

        // don't allow staff to login without 2FA set up
        ElevationLevel::Staff if secret.is_none() => {
            log::error!("Staff user {user_id} tried to login without 2FA enabled");

            return Err(Error::NotFound);
        }
        _ => {}
    }

    if flags.contains(UserFlags::BANNED) {
        return Err(Error::Banned);
    }

    if secret.is_some() != backup.is_some() {
        return Err(Error::InternalErrorStatic("MFA Mismatch!"));
    }

    // early check, before any work is done
    if secret.is_some() && form.totp.is_none() {
        return Err(Error::TOTPRequired);
    }

    if !verify_password(&state, passhash, form.password).await? {
        return Err(Error::InvalidCredentials);
    }

    if let (Some(secret), Some(backup)) = (secret, backup) {
        if !process_2fa(&state, user_id, secret, backup, &form.totp.unwrap()).await? {
            return Err(Error::InvalidCredentials);
        }
    }

    do_login(state, addr, user_id, std::time::SystemTime::now()).await
}

pub async fn do_login(
    state: ServerState,
    addr: SocketAddr,
    user_id: Snowflake,
    now: std::time::SystemTime,
) -> Result<Session, Error> {
    let token = RawAuthToken::random_bearer();
    let bytes = match token {
        RawAuthToken::Bearer(ref bytes) => &bytes[..],
        _ => unreachable!(),
    };

    let expires = now + state.config().account.session_duration;
    let ip = addr.ip();

    let db = state.db.write.get().await?;

    db.execute2(schema::sql! {
        INSERT INTO Sessions (
            Token, UserId, Expires, Addr
        ) VALUES (
            #{&bytes    as Sessions::Token   },
            #{&user_id  as Sessions::UserId  },
            #{&expires  as Sessions::Expires },
            #{&ip       as Sessions::Addr    }
        )
    })
    .await?;

    Ok(Session {
        auth: token.into(),
        expires: expires.into(),
    })
}

pub async fn verify_password(
    state: &ServerState,
    passhash: &str,
    password: smol_str::SmolStr,
) -> Result<bool, Error> {
    // NOTE: Given how expensive it can be to compute an argon2 hash,
    // this only allows a given number to process at once.
    let _permit =
        state.mem_semaphore.acquire_many(crate::backend::api::user::register::hash_memory_cost()).await?;

    // SAFETY: This is only used within the following spawn_blocking block,
    // but will remain alive until `drop(user)` below.
    let passhash: &'static str = unsafe { std::mem::transmute(passhash) };

    let verified = tokio::task::spawn_blocking(move || {
        let config = crate::backend::api::user::register::hash_config();
        argon2::verify_encoded_ext(passhash, password.as_bytes(), config.secret, config.ad)
    })
    .await??;

    drop(_permit);

    Ok(verified)
}

fn validate_2fa_token(token: &str) -> Result<(), Error> {
    match token.len() {
        6 => {
            if !token.chars().all(|c: char| c.is_ascii_digit()) {
                return Err(Error::TOTPRequired);
            }
        }
        13 => {
            // Taken from base32::Alphabet::Crockford
            if !token.bytes().all(|c: u8| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&c)) {
                return Err(Error::TOTPRequired);
            }
        }
        _ => return Err(Error::TOTPRequired),
    }

    Ok(())
}

pub async fn process_2fa(
    state: &ServerState,
    user_id: Snowflake,
    encrypted_secret: &[u8],
    encrypted_backup: &[u8],
    token: &str,
) -> Result<bool, Error> {
    // User's cannot do multiple 2fa requests at once
    let _guard = state.id_lock.lock(user_id).await;

    let mfa_key = state.config().keys.mfa_key;

    match token.len() {
        // 6-digit TOTP code
        6 => {
            let Ok(token) = token.parse() else {
                return Err(Error::InvalidCredentials);
            };

            let Some(secret) = decrypt_user_message(&mfa_key, user_id, encrypted_secret) else {
                return Err(Error::InternalErrorStatic("Decrypt Error"));
            };

            let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();

            // Get the last used timestamp for this user's TOTP, or default to 0
            //
            // NOTE: Without the `id_lock _guard`, this might introduce security issues given `peek_with` is not linearizable,
            //  however, with the lock, this will never happen.
            let mut last = state.mfa_last.peek_with(&user_id, |_, last| *last).unwrap_or_default();

            if !totp::TOTP6::new(&secret).check(token, now, &mut last) {
                return Err(Error::InvalidCredentials);
            }

            use scc::hash_index::Entry;

            match state.mfa_last.entry_async(user_id).await {
                Entry::Occupied(entry) => entry.update(last),
                Entry::Vacant(entry) => {
                    entry.insert_entry(last);
                }
            }
        }

        // 13-character backup code
        13 => {
            let token = match base32::decode(base32::Alphabet::Crockford, token) {
                Some(token) if token.len() == 8 => token,
                _ => return Err(Error::InvalidCredentials),
            };

            let mut backup = match decrypt_user_message(&mfa_key, user_id, encrypted_backup) {
                Some(backup) if backup.len() % 8 == 0 => backup,
                _ => return Err(Error::InternalErrorStatic("Decrypt Error")),
            };

            let Some(idx) = backup.chunks_exact(8).position(|code| code == token) else {
                return Err(Error::InvalidCredentials);
            };

            log::debug!("MFA Backup token used, saving new backup array to database");

            // fill old backup code with randomness to prevent reuse
            util::rng::crypto_thread_rng().fill_bytes({
                let start = idx * 8;
                &mut backup[start..start + 8]
            });

            let backup = encrypt_user_message(&mfa_key, user_id, &backup);

            #[rustfmt::skip]
            state.db.write.get().await?.execute2(schema::sql! {
                UPDATE Users SET (MfaBackup) = (#{&backup as Users::MfaBackup})
                 WHERE Users.Id = #{&user_id as Users::Id}
            })
            .await?;
        }
        _ => return Err(Error::InvalidCredentials),
    }

    drop(_guard);

    Ok(true)
}
