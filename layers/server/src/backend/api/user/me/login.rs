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

// TODO: Determine if I should give any feedback at all or
// just say catchall "invalid username/email/password"
pub async fn login(state: ServerState, addr: SocketAddr, form: UserLoginForm) -> Result<Session, Error> {
    validate_email(&form.email)?;

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

    let elevation = flags.elevation();

    // System user flat out cannot log in. Pretend it doesn't exist.
    if elevation == ElevationLevel::System {
        return Err(Error::NotFound);
    }

    // don't allow staff to login without 2FA
    if elevation == ElevationLevel::Staff && secret.is_none() {
        log::error!("Staff user {user_id} tried to login without 2FA enabled");

        return Err(Error::NotFound);
    }

    if flags.contains(UserFlags::BANNED) {
        return Err(Error::Banned);
    }

    if secret.is_some() != backup.is_some() {
        return Err(Error::InternalErrorStatic("Secret/Backup Mismatch!"));
    }

    let verified = {
        let _permit =
            state.mem_semaphore.acquire_many(crate::backend::api::user::register::hash_memory_cost()).await?;

        // SAFETY: This is only used within the following spawn_blocking block,
        // but will remain alive until `drop(user)` below.
        let passhash: &'static str = unsafe { std::mem::transmute(passhash) };

        let password = form.password;
        let verified = tokio::task::spawn_blocking(move || {
            let config = crate::backend::api::user::register::hash_config();
            argon2::verify_encoded_ext(passhash, password.as_bytes(), config.secret, config.ad)
        })
        .await??;

        drop(_permit);

        verified
    };

    if !verified {
        return Err(Error::InvalidCredentials);
    }

    if let (Some(secret), Some(backup)) = (secret, backup) {
        let Some(token) = form.totp else { return Err(Error::TOTPRequired); };

        if !process_2fa(&state, user_id, secret, backup, &token).await? {
            return Err(Error::InvalidCredentials);
        }
    }

    drop(user);

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

pub async fn process_2fa(
    state: &ServerState,
    user_id: Snowflake,
    secret: &[u8],
    backup: &[u8],
    token: &str,
) -> Result<bool, Error> {
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();

    // TODO: Create a global cache for user_id -> last TOTP step to prevent reuse. This value is only useful for 30 seconds,
    // so we can clear the cache after like 60.
    let mut last = 0;

    let mfa_key = state.config().keys.mfa_key;

    let Some(secret) = decrypt_user_message(&mfa_key, user_id, secret) else {
        return Err(Error::InternalErrorStatic("Decrypt Error!"));
    };

    match token.len() {
        // 6-digit TOTP code
        6 => totp::TOTP6::new(&secret).check_str(token, now, &mut last).map_err(|_| Error::InvalidCredentials),

        // 13-character backup code
        13 => {
            let token = match base32::decode(base32::Alphabet::Crockford, token) {
                Some(token) if token.len() == 8 => token,
                _ => return Err(Error::InvalidCredentials),
            };

            let mut backup = match decrypt_user_message(&mfa_key, user_id, backup) {
                Some(backup) if backup.len() % 8 == 0 => backup,
                _ => return Err(Error::InternalErrorStatic("Decrypt Error!")),
            };

            if let Some(idx) = backup.chunks_exact(8).position(|code| code == token) {
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

                Ok(true)
            } else {
                Err(Error::InvalidCredentials)
            }
        }
        _ => Err(Error::InvalidCredentials),
    }
}
