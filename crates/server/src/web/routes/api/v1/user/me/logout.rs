use std::borrow::Cow;
use std::sync::Arc;

use db::{Client, ClientError, Snowflake};

use crate::ctrl::auth;
use crate::ServerState;

use ftl::*;

pub async fn logout(route: Route<ServerState>, auth: auth::Authorization) -> impl Reply {
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
        .execute_cached_typed(|| delete_session(), &[&auth.token.bytes()])
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

use thorn::*;

fn delete_session() -> impl AnyQuery {
    use db::schema::*;

    Query::delete()
        .from::<Sessions>()
        .and_where(Sessions::Token.equals(Var::of(Sessions::Token)))
}
