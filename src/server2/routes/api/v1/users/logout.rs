use std::borrow::Cow;
use std::sync::Arc;

use http::StatusCode;

use auth::authorize;

use crate::{
    db::{Client, ClientError, Snowflake},
    server2::{
        auth::{self, AuthError, AuthToken},
        rate::RateLimitKey,
        ServerState,
    },
};

use super::{Reply, Route};

pub async fn logout(mut route: Route) -> impl Reply {
    let auth = match auth::authorize(&route).await {
        Ok(auth) => auth,
        Err(e) => return e.into_response(),
    };

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
