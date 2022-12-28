use super::*;

#[async_recursion]
pub async fn get_profile(
    route: Route<ServerState>,
    auth: Authorization,
    member_id: Snowflake,
    party_id: Snowflake,
) -> WebResult {
    Ok(WebResponse::new(
        crate::backend::api::user::profile::get_profile(route.state, auth, member_id, Some(party_id)).await?,
    ))
}

#[async_recursion]
pub async fn patch_profile(route: Route<ServerState>, auth: Authorization, party_id: Snowflake) -> WebResult {
    unimplemented!()
}
