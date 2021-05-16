use db::{ClientError, Snowflake};

use crate::ServerState;

#[derive(Deserialize)]
pub struct LoginForm {
    email: String,
    password: String,
}

use crate::routes::api::auth::AuthToken;
use ftl::*;

pub async fn login(mut route: Route<ServerState>) -> impl Reply {
    let form = match body::any::<LoginForm, _>(&mut route).await {
        Ok(form) => form,
        Err(e) => return e.into_response(),
    };

    match login_user(route.state, form).await {
        Ok(ref session) => reply::json(session).into_response(),
        Err(e) => match e {
            LoginError::ClientError(_)
            | LoginError::JoinError(_)
            | LoginError::SemaphoreError(_)
            | LoginError::PasswordHashError(_) => {
                log::error!("Login Error {}", e);

                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
            _ => e
                .to_string()
                .with_status(StatusCode::BAD_REQUEST)
                .into_response(),
        },
    }
}

// TODO: Determine if I should give any feedback at all or
// just say catchall "invalid username/email/password"
#[derive(thiserror::Error, Debug)]
enum LoginError {
    #[error("Invalid Email or Password")]
    InvalidCredentials,

    #[error("Database Error {0}")]
    ClientError(#[from] ClientError),

    #[error("Join Error {0}")]
    JoinError(#[from] tokio::task::JoinError),

    #[error("Password Hash Error {0}")]
    PasswordHashError(#[from] argon2::Error),

    #[error("Semaphore Error: {0}")]
    SemaphoreError(#[from] tokio::sync::AcquireError),
}

use super::super::register::{hash_config, EMAIL_REGEX};

async fn login_user(state: ServerState, mut form: LoginForm) -> Result<Session, LoginError> {
    if !EMAIL_REGEX.is_match(&form.email) {
        return Err(LoginError::InvalidCredentials);
    }

    let user = state
        .db
        .read
        .query_opt_cached(
            || "SELECT id, email, passhash, deleted_at FROM lantern.users WHERE email = $1",
            &[&form.email],
        )
        .await?;

    let user = match user {
        Some(user) => user,
        None => return Err(LoginError::InvalidCredentials),
    };

    let id: Snowflake = user.get(0);
    let passhash: String = user.get(2);
    let deleted_at: Option<time::PrimitiveDateTime> = user.get(3);

    if deleted_at.is_some() {
        return Err(LoginError::InvalidCredentials);
    }

    // NOTE: Given how expensive it can be to compute an argon2 hash,
    // this only allows a given number to process at once.
    let permit = state.hashing_semaphore.acquire().await?;

    let verified = tokio::task::spawn_blocking(move || {
        let config = hash_config();
        argon2::verify_encoded_ext(
            &passhash,
            form.password.as_bytes(),
            config.secret,
            config.ad,
        )
    })
    .await??;

    drop(permit);

    if !verified {
        return Err(LoginError::InvalidCredentials);
    }

    Ok(do_login(state, id, std::time::SystemTime::now()).await?)
}

#[derive(Clone, Debug, Serialize)]
pub struct Session {
    auth: String,
    expires: String,
}

pub async fn do_login(
    state: ServerState,
    id: Snowflake,
    now: std::time::SystemTime,
) -> Result<Session, ClientError> {
    let token = AuthToken::new();

    let expires = now + state.config.login_session_duration;

    state
        .db
        .write
        .execute_cached(
            || "INSERT INTO lantern.sessions (token, user_id, expires) VALUES ($1, $2, $3)",
            &[&&token.0[..], &id, &expires],
        )
        .await?;

    Ok(Session {
        auth: token.encode(),
        expires: time::OffsetDateTime::from(expires).format(time::Format::Rfc3339),
    })
}
