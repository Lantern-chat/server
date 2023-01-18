use super::*;

#[async_recursion]
pub async fn get_members(route: Route<ServerState>, auth: Authorization, party_id: Snowflake) -> WebResult {
    Ok(WebResponse::stream(
        crate::backend::api::party::members::get_many(route.state, auth, party_id).await?,
    ))
}

#[async_recursion]
pub async fn get_member(
    route: Route<ServerState>,
    auth: Authorization,
    party_id: Snowflake,
    member_id: Snowflake,
) -> WebResult {
    Ok(WebResponse::new(
        crate::backend::api::party::members::get_one(route.state, auth, party_id, member_id, true).await?,
    ))
}
