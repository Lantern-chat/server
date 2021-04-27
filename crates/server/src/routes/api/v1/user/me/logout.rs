use std::borrow::Cow;
use std::sync::Arc;

use db::{Client, ClientError, Snowflake};

use crate::ServerState;

use crate::routes::api::auth;
use ftl::*;

pub async fn logout(mut route: Route<ServerState>, auth: auth::Authorization) -> impl Reply {
    if let Err(e) = logout_user(route.state, auth).await {
        log::error!("Logout error: {}", e);
    }

    StatusCode::OK.into_response()
}

#[derive(Debug, thiserror::Error)]
enum LogoutError {
    #[error("Database Error {0}")]
    ClientError(#[from] ClientError),
}

async fn logout_user(state: ServerState, auth: auth::Authorization) -> Result<(), LogoutError> {
    let res = state
        .db
        .write
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
