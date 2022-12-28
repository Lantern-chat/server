use super::*;

#[async_recursion]
pub async fn get_room(route: Route<ServerState>, auth: Authorization, room_id: Snowflake) -> WebResult {
    Ok(WebResponse::new(
        crate::backend::api::room::get::get_room(route.state, auth, room_id).await?,
    ))
}
