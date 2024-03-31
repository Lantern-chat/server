use sdk::api::commands::party::{GetPartyMember, GetPartyMembers};

use super::*;

#[async_recursion]
pub async fn get_members(route: Route<ServerState>, _auth: Authorization, party_id: Snowflake) -> ApiResult {
    Ok(Procedure::from(GetPartyMembers { party_id }))
}

#[async_recursion]
pub async fn get_member(
    route: Route<ServerState>,
    _auth: Authorization,
    party_id: Snowflake,
    member_id: Snowflake,
) -> ApiResult {
    Ok(Procedure::from(GetPartyMember { party_id, member_id }))
}
