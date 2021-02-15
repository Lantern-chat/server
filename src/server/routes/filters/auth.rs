use std::{convert::Infallible, net::SocketAddr, str::FromStr, sync::Arc};

use warp::{
    hyper::{Server, StatusCode},
    reject::Reject,
    Filter, Rejection, Reply,
};

use crate::{
    db::{ClientError, Snowflake},
    server::{
        auth::{AuthToken, AuthTokenFromStrError},
        rate::RateLimitKey,
        routes::api::ApiError,
        ServerState,
    },
};

#[derive(Debug)]
pub struct NoAuth;
impl Reject for NoAuth {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Authorization {
    pub token: AuthToken,
    pub user_id: Snowflake,
}

pub fn auth(
    state: ServerState,
) -> impl Filter<Extract = (Authorization,), Error = Rejection> + Clone {
    warp::header::<String>("Authorization")
        .and(state.inject())
        .and_then(|auth, state| async move {
            match authorize(auth, state).await {
                Ok(sf) => Ok(sf),
                Err(e) => {
                    // only log important errors
                    if let AuthError::DecodeError(_) | AuthError::ClientError(_) = e {
                        log::error!("{}", e)
                    }

                    Err(warp::reject::custom(NoAuth))
                }
            }
        })
}

pub fn no_auth(
    _err: Rejection,
) -> impl Filter<Extract = (impl Reply,), Error = Infallible> + Clone {
    warp::any().map(|| ApiError::reply_json(StatusCode::UNAUTHORIZED, "UNAUTHORIZED"))
}

use std::convert::TryInto;

#[derive(Debug, thiserror::Error)]
enum AuthError {
    #[error("No Session")]
    NoSession,

    #[error("Invalid Format")]
    InvalidFormat,

    #[error("Decode Error: {0}")]
    DecodeError(#[from] base64::DecodeError),

    #[error("Client Error: {0}")]
    ClientError(#[from] ClientError),

    #[error("Auth Token Parse Error: {0}")]
    AuthTokenParseError(#[from] AuthTokenFromStrError),
}

async fn authorize(header: String, state: ServerState) -> Result<Authorization, AuthError> {
    const BEARER: &'static str = "Bearer ";

    if (!header.starts_with(BEARER)) {
        return Err(AuthError::InvalidFormat);
    }

    let token = AuthToken::from_str(&header[BEARER.len()..])?;

    // TODO: Cache this
    let session = state
        .db
        .query_opt_cached(
            || "SELECT user_id, expires FROM lantern.sessions WHERE token = $1",
            &[&&token.0[..]],
        )
        .await?;

    match session {
        Some(row) => {
            let expires: std::time::SystemTime = row.get(1);

            if expires <= std::time::SystemTime::now() {
                return Err(AuthError::NoSession);
            }

            Ok(Authorization {
                token,
                user_id: row.get(0),
            })
        }
        None => Err(AuthError::NoSession),
    }
}
