use super::*;

#[async_recursion]
pub async fn trigger_typing(mut route: Route<ServerState>, auth: Authorization, room_id: Snowflake) -> WebResult {
    let body = body::any(&mut route).await.unwrap_or_default();

    crate::backend::api::room::typing::trigger_typing(route.state, auth, room_id, body).await?;

    Ok(StatusCode::NO_CONTENT.into())
}
