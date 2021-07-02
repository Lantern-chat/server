use ftl::*;

use db::Snowflake;

use crate::ctrl::Error;
use crate::web::{auth::Authorization, routes::api::ApiError};

pub async fn trigger_typing(
    route: Route<crate::ServerState>,
    auth: Authorization,
    room_id: Snowflake,
) -> impl Reply {
    match crate::ctrl::room::typing::trigger_typing(route.state, auth, room_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => ApiError::err(e).into_response(),
    }
}