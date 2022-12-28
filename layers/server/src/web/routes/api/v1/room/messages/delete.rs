use super::*;

#[async_recursion]
pub async fn delete(
    route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
) -> WebResult {
    crate::backend::api::room::messages::delete::delete_msg(route.state, auth, room_id, msg_id).await?;

    Ok(StatusCode::OK.into())
}
