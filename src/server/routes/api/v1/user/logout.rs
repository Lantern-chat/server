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
        routes::{api::ApiError, filters::auth},
        ServerState,
    },
};

pub fn logout(
    state: Arc<ServerState>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::delete()
        .and(warp::path("logout"))
        .and(auth(state.clone()))
        .map(move |auth| (state.clone(), auth))
        .and_then(|(state, auth)| async move {
            if let Err(e) = logout_user(state, auth).await {
                log::error!("Logout error: {}", e);
            }

            Ok::<_, Rejection>(warp::reply::reply())
        })
}

#[derive(Debug, thiserror::Error)]
enum LogoutError {
    #[error("Database Error {0}")]
    ClientError(#[from] ClientError),
}

async fn logout_user(state: Arc<ServerState>, auth: auth::Auth) -> Result<(), LogoutError> {
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
