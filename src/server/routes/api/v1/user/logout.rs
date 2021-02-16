use std::borrow::Cow;
use std::sync::Arc;

use warp::{
    body::json,
    hyper::{Server, StatusCode},
    reject::Reject,
    Filter, Rejection, Reply,
};

use crate::{
    db::{Client, ClientError, Snowflake},
    server::{
        auth::AuthToken,
        rate::RateLimitKey,
        routes::{
            error::ApiError,
            filters::{auth, no_auth},
        },
        ServerState,
    },
};

pub fn logout(
    state: ServerState,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    auth(state.clone())
        .and(state.inject())
        .and_then(|auth, state| async move {
            if let Err(e) = logout_user(state, auth).await {
                log::error!("Logout error: {}", e);
            }

            Ok::<_, Rejection>(warp::reply::reply())
        })
        .recover(ApiError::recover)
}

#[derive(Debug, thiserror::Error)]
enum LogoutError {
    #[error("Database Error {0}")]
    ClientError(#[from] ClientError),
}

async fn logout_user(state: ServerState, auth: auth::Authorization) -> Result<(), LogoutError> {
    let res = state
        .db
        .execute_cached(
            || "DELETE FROM lantern.sessions WHERE token = $1",
            &[&auth.token.bytes()],
        )
        .await?;

    if res == 0 {
        log::warn!(
            "Attempted to delete nonexistent session: {}, user: {}",
            auth.token.encode(),
            auth.user_id
        );
    }

    Ok(())
}
