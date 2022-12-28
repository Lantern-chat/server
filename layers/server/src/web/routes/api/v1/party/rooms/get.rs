use super::*;

#[async_recursion]
pub async fn get(route: Route<crate::ServerState>, auth: Authorization, party_id: Snowflake) -> WebResult {
    Ok(WebResponse::new(
        crate::backend::api::party::rooms::get::get_rooms(route.state, auth, party_id).await?,
    ))
}
