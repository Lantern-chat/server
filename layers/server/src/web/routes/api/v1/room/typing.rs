use super::*;

#[async_recursion]
pub async fn trigger_typing(route: Route<ServerState>, auth: Authorization, room_id: Snowflake) -> WebResult {
    crate::backend::api::room::typing::trigger_typing(route.state, auth, room_id).await?;

    Ok(StatusCode::NO_CONTENT.into())
}
