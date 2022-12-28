use super::*;

#[async_recursion]
pub async fn get(route: Route<ServerState>, auth: Authorization, party_id: Snowflake) -> WebResult {
    Ok(WebResponse::new(
        crate::backend::api::party::get::get_party(route.state, auth, party_id).await?,
    ))
}
