use super::*;

#[async_recursion]
pub async fn get(
    route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    thread_id: Snowflake,
) -> WebResult {
    Ok(WebResponse::new(
        crate::backend::api::room::threads::get::get_thread(route.state, auth, room_id, thread_id).await?,
    ))
}
