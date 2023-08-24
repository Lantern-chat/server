use std::{net::SocketAddr, time::SystemTime};

use schema::{auth::RawAuthToken, Snowflake};

use crate::backend::{api::auth::AuthTokenExt, util::validation::validate_email};
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

    #[rustfmt::skip]
    let user = state.db.read.get().await?.query_opt2(schema::sql! {
        SELECT
            Users.Id        AS @Id,
            Users.Flags     AS @Flags,
            Users.Passhash  AS @Passhash,
            Users.Mfa       AS @Mfa
        FROM  LiveUsers AS Users
        WHERE Users.Email = #{&form.email as Users::Email}
    }).await?;

    let Some(user) = user else {
        return Err(Error::InvalidCredentials);
    };

    let user_id: Snowflake = user.id()?;
    let flags = UserFlags::from_bits_truncate(user.flags()?);
    let passhash = user.passhash()?;
    let mfa: Option<&[u8]> = user.mfa()?;

    match flags.elevation() {
        // System user flat out cannot log in. Pretend it doesn't exist.
        ElevationLevel::System => return Err(Error::NotFound),

        // don't allow staff to login without 2FA set up
        ElevationLevel::Staff if mfa.is_none() => {
            log::error!("Staff user {user_id} tried to login without 2FA enabled");

            return Err(Error::NotFound);
        }
        _ => {}
    }

    if flags.contains(UserFlags::BANNED) {
        return Err(Error::Banned);
    }

    // early check, before any work is done
    if mfa.is_some() && form.totp.is_none() {
        return Err(Error::TOTPRequired);
    }

    if !verify_password(&state, passhash, &form.password).await? {
        return Err(Error::InvalidCredentials);
    }

    if let Some(mfa) = mfa {
        if !process_2fa(
            &state,
            user_id,
            ProvidedMfa::Encrypted(mfa),
            &form.password,
            &form.totp.unwrap(),
        )
        .await?
        {
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

pub async fn verify_password(state: &ServerState, passhash: &str, password: &str) -> Result<bool, Error> {
    // NOTE: Given how expensive it can be to compute an argon2 hash,
    // this only allows a given number to process at once.
    let _permit =
        state.mem_semaphore.acquire_many(crate::backend::api::user::register::hash_memory_cost()).await?;

    // SAFETY: These are only used within the following spawn_blocking block
    let passhash: &'static str = unsafe { std::mem::transmute(passhash) };
    let password: &'static str = unsafe { std::mem::transmute(password) };

    let verified = tokio::task::spawn_blocking(|| {
        let config = crate::backend::api::user::register::hash_config();
        argon2::verify_encoded_ext(passhash, password.as_bytes(), config.secret, config.ad)
    })
    .await??;

    drop(_permit);

    Ok(verified)
}

pub fn validate_2fa_token(token: &str) -> Result<(), Error> {
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

enum MfaAttempt {
    Token(u32),
    Backup(u64),
}

impl MfaAttempt {
    pub fn parse(token: &str) -> Result<Self, Error> {
        match token.len() {
            // 6-digit TOTP code
            6 => match token.parse() {
                Ok(token) => Ok(MfaAttempt::Token(token)),
                Err(_) => Err(Error::InvalidCredentials),
            },
            // 13-character backup code
            13 => match base32::decode(base32::Alphabet::Crockford, token) {
                Some(token_bytes) if token_bytes.len() == 8 => {
                    let mut token = [0; 8];
                    token.copy_from_slice(&token_bytes);
                    Ok(MfaAttempt::Backup(u64::from_le_bytes(token)))
                }
                _ => Err(Error::InvalidCredentials),
            },
            _ => Err(Error::InvalidCredentials),
        }
    }
}

pub enum ProvidedMfa<'a> {
    Encrypted(&'a [u8]),
    Plain(&'a mfa_totp::MFA),
}

pub async fn process_2fa<'a>(
    state: &ServerState,
    user_id: Snowflake,
    mfa: ProvidedMfa<'a>,
    password: &str,
    token: &str,
) -> Result<bool, Error> {
    use crate::backend::util::encrypt::nonce_from_user_id;
    use mfa_totp::{totp::TOTP6, MFA};
    use rand::Rng;

    // User's cannot do multiple 2fa requests at once
    let _guard = state.id_lock.lock(user_id).await;

    let token = MfaAttempt::parse(token)?;

    let mfa_key = state.config().keys.mfa_key;

    let nonce = nonce_from_user_id(user_id);

    let mfa = match mfa {
        ProvidedMfa::Plain(mfa) => *mfa,
        ProvidedMfa::Encrypted(encrypted_mfa) => match MFA::decrypt(&mfa_key, &nonce, password, encrypted_mfa) {
            Ok(mfa) => mfa,
            Err(_) => return Err(Error::InternalErrorStatic("Decrypt Error")),
        },
    };

    match token {
        MfaAttempt::Token(token) => {
            let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();

            // Get the last used timestamp for this user's TOTP, or default to 0
            //
            // NOTE: Without the `id_lock _guard`, this might introduce security issues given `peek_with` is not linearizable,
            //  however, with the lock, this will never happen.
            let mut last = state.mfa_last.peek_with(&user_id, |_, last| *last).unwrap_or_default();

            if !TOTP6::new(&mfa.key).check(token, now, &mut last) {
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
        MfaAttempt::Backup(code) => {
            let mut mfa = mfa; // make mutable

            let Some(idx) = mfa.backups.iter().position(|backup| code == *backup) else {
                return Err(Error::InvalidCredentials);
            };

            log::debug!("MFA Backup token used, saving new backup to database");

            // set old backup code to random value to prevent reuse
            mfa.backups[idx] = util::rng::crypto_thread_rng().gen();

            let Ok(encrypted_mfa) = mfa.encrypt(&mfa_key, &nonce, password) else {
                return Err(Error::InternalErrorStatic("Encrypt error"));
            };

            #[rustfmt::skip]
            state.db.write.get().await?.execute2(schema::sql! {
                UPDATE Users SET (Mfa) = (#{&encrypted_mfa as Users::Mfa})
                 WHERE Users.Id = #{&user_id as Users::Id}
            })
            .await?;
        }
    }

    drop(_guard);

    Ok(true)
}
