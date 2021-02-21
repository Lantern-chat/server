use std::borrow::Cow;
use std::sync::Arc;

use http::StatusCode;

use crate::{
    db::{ClientError, Snowflake},
    server::{
        ftl::{
            body::{content_length_limit, form, BodyDeserializeError},
            rate_limit::RateLimitKey,
            reply,
        },
        routes::api::util::time::is_of_age,
        ServerState,
    },
};

#[derive(Deserialize)]
pub struct LoginForm {
    email: String,
    password: String,
}

use crate::server::ftl::*;
use crate::server::routes::api::auth::AuthToken;

pub async fn login(mut route: Route) -> impl Reply {
    // 10KB max form size
    if let Some(err) = content_length_limit(&route, 1024 * 10) {
        return err.into_response();
    }

    let form = match form::<LoginForm>(&mut route).await {
        Ok(form) => form,
        Err(e) => return e.into_response(),
    };

    match login_user(route.state, form).await {
        Ok(ref session) => reply::json(session).into_response(),
        Err(e) => match e {
            LoginError::ClientError(_)
            | LoginError::JoinError(_)
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

/*
pub fn login(
    state: ServerState,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    state
        .inject()
        .and(warp::body::form::<LoginForm>())
        .and_then(|state: ServerState, form: LoginForm| async move {
            Ok::<_, Rejection>(match login_user(state, form).await {
                Ok(ref session) => ApiError::ok(session),
                Err(ref e) => match e {
                    LoginError::ClientError(_)
                    | LoginError::JoinError(_)
                    | LoginError::PasswordHashError(_) => {
                        log::error!("{}", e);

                        ApiError::err(StatusCode::INTERNAL_SERVER_ERROR, "Internal Error".into())
                    }
                    _ => ApiError::err(StatusCode::BAD_REQUEST, e.to_string().into()),
                },
            })
        })
        .recover(ApiError::recover)
}
*/

// TODO: Determine if I should give any feedback at all or
// just say catchall "invalid username/email/password"
#[derive(thiserror::Error, Debug)]
enum LoginError {
    #[error(transparent)]
    BodyDeserializeError(#[from] BodyDeserializeError),

    #[error("Invalid Email or Password")]
    InvalidCredentials,

    #[error("Database Error {0}")]
    ClientError(#[from] ClientError),

    #[error("Join Error {0}")]
    JoinError(#[from] tokio::task::JoinError),

    #[error("Password Hash Error {0}")]
    PasswordHashError(#[from] argon2::Error),
}

use super::register::{hash_config, EMAIL_REGEX};

async fn login_user(state: ServerState, mut form: LoginForm) -> Result<Session, LoginError> {
    if !EMAIL_REGEX.is_match(&form.email) {
        return Err(LoginError::InvalidCredentials);
    }

    let user = state
        .db
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

    let expires = now + std::time::Duration::from_secs(90 * 24 * 60 * 60); // TODO: Set from config

    state
        .db
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
