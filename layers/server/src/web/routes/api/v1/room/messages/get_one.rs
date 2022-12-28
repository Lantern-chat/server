use super::*;

#[async_recursion]
pub async fn get_one(
    route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
) -> WebResult {
    Ok(WebResponse::new(
        crate::backend::api::room::messages::get::get_one(route.state, auth, room_id, msg_id).await?,
    ))
}
