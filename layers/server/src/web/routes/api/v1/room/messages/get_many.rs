use super::*;

#[async_recursion]
pub async fn get_many(route: Route<crate::ServerState>, auth: Authorization, room_id: Snowflake) -> WebResult {
    let form = match route.query() {
        None => Default::default(),
        Some(form) => form?,
    };

    Ok(WebResponse::stream(
        crate::backend::api::room::messages::get::get_many(route.state, auth, room_id, form).await?,
    ))
}
