use super::*;

#[async_recursion]
pub async fn get_members(route: Route<ServerState>, auth: Authorization, party_id: Snowflake) -> WebResult {
    Ok(WebResponse::stream(
        crate::backend::api::party::members::get_members(route.state, party_id, auth.user_id).await?,
    ))
}
