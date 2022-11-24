use ftl::*;

use schema::Snowflake;

use super::ApiResponse;
use crate::Authorization;

#[async_recursion]
pub async fn trigger_typing(
    route: Route<crate::ServerState>,
    auth: Authorization,
    room_id: Snowflake,
) -> ApiResponse {
    crate::backend::api::room::typing::trigger_typing(
        route.state,
        auth,
        room_id,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}
