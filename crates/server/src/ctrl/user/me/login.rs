use std::{alloc::System, net::SocketAddr, time::SystemTime};

use schema::Snowflake;

use crate::{
    ctrl::{auth::AuthToken, Error},
    util::encrypt::decrypt_user_message,
    ServerState,
};

#[derive(Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,

    #[serde(default)]
    pub totp: Option<String>,
}

use models::Session;

use crate::ctrl::user::register::{hash_config, EMAIL_REGEX};

// TODO: Determine if I should give any feedback at all or
// just say catchall "invalid username/email/password"
pub async fn login(state: ServerState, addr: SocketAddr, form: LoginForm) -> Result<Session, Error> {
    if !EMAIL_REGEX.is_match(&form.email) {
        return Err(Error::InvalidCredentials);
    }

    let db = state.db.read.get().await?;

    let user = db
        .query_opt_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .from_table::<Users>()
                    .cols(&[Users::Id, Users::Passhash, Users::MfaSecret, Users::MfaBackup])
                    .and_where(Users::Email.equals(Var::of(Users::Email)))
                    .and_where(Users::DeletedAt.is_null())
            },
            &[&form.email],
        )
        .await?;

    let user = match user {
        Some(user) => user,
        None => return Err(Error::InvalidCredentials),
    };

    let user_id: Snowflake = user.try_get(0)?;
    let passhash: String = user.try_get(1)?;
    let secret: Option<Vec<u8>> = user.try_get(2)?;
    let backup: Option<Vec<u8>> = user.try_get(3)?;

    if secret.is_some() != backup.is_some() {
        return Err(Error::InternalErrorStatic("Secret/Backup Mismatch!"));
    }

    let verified = {
        // NOTE: Given how expensive it can be to compute an argon2 hash,
        // this only allows a given number to process at once.
        let permit = state.hashing_semaphore.acquire().await?;

        let password = form.password;
        let verified = tokio::task::spawn_blocking(move || {
            let config = hash_config();
            argon2::verify_encoded_ext(&passhash, password.as_bytes(), config.secret, config.ad)
        })
        .await??;

        drop(permit);

        verified
    };

    if !verified {
        return Err(Error::InvalidCredentials);
    }

    if let (Some(secret), Some(backup)) = (secret, backup) {
        match form.totp {
            None => return Err(Error::TOTPRequired),
            Some(token) => {
                if !process_2fa(&state, user_id, secret, backup, token).await? {
                    return Err(Error::InvalidCredentials);
                }
            }
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
    let token = AuthToken::new();

    let expires = now + state.config.login_session_duration;

    state
        .write_db()
        .await
        .execute_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::insert()
                    .into::<Sessions>()
                    .cols(&[
                        Sessions::Token,
                        Sessions::UserId,
                        Sessions::Expires,
                        Sessions::Addr,
                    ])
                    .values(vec![
                        Var::of(Sessions::Token),
                        Var::of(Sessions::UserId),
                        Var::of(Sessions::Expires),
                        Var::of(Sessions::Addr),
                    ])
            },
            &[&&token.0[..], &user_id, &expires, &addr.ip()],
        )
        .await?;

    Ok(Session {
        auth: token.encode(),
        expires: chrono::DateTime::<chrono::Utc>::from(expires)
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    })
}

pub async fn process_2fa(
    state: &ServerState,
    user_id: Snowflake,
    secret: Vec<u8>,
    backup: Vec<u8>,
    token: String,
) -> Result<bool, Error> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let secret = match decrypt_user_message(&state.config.mfa_key, user_id, &secret) {
        Ok(secret) => secret,
        Err(_) => return Err(Error::InternalErrorStatic("Decrypt Error!")),
    };

    match token.len() {
        6 => {
            // TODO: Account for time skew in the check method
            if Ok(true) != totp::TOTP6::new(&secret).check_str(&token, now) {
                return Ok(false);
            }
        }
        13 => {
            let _backup = match decrypt_user_message(&state.config.mfa_key, user_id, &backup) {
                Ok(backup) => backup,
                Err(_) => return Err(Error::InternalErrorStatic("Decrypt Error!")),
            };

            return Ok(false);
            // TODO: Backup codes
        }
        _ => return Err(Error::InvalidCredentials),
    }

    Ok(true)
}
