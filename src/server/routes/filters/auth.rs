use std::{net::SocketAddr, str::FromStr, sync::Arc};

use warp::{hyper::Server, reject::Reject, Filter, Rejection, Reply};

use crate::{
    db::{ClientError, Snowflake},
    server::{
        auth::{AuthToken, AuthTokenFromStrError},
        rate::RateLimitKey,
        ServerState,
    },
};

#[derive(Debug)]
pub struct NoAuth;
impl Reject for NoAuth {}

pub fn auth(
    state: Arc<ServerState>,
) -> impl Filter<Extract = (Snowflake,), Error = Rejection> + Clone {
    warp::header::<String>("Authorization")
        .map(move |addr| (addr, state.clone()))
        .and_then(|(auth, state)| async move {
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

async fn authorize(header: String, state: Arc<ServerState>) -> Result<Snowflake, AuthError> {
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

            Ok(row.get(0))
        }
        None => Err(AuthError::NoSession),
    }
}
