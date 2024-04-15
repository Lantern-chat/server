use sdk::api::commands::party::{GetPartyMember, GetPartyMembers};

use super::*;

#[async_recursion]
pub async fn get_members(route: Route<ServerState>, _auth: Authorization, party_id: PartyId) -> ApiResult {
    Ok(Procedure::from(GetPartyMembers { party_id }))
}

#[async_recursion]
pub async fn get_member(
    route: Route<ServerState>,
    _auth: Authorization,
    party_id: PartyId,
    member_id: UserId,
) -> ApiResult {
    Ok(Procedure::from(GetPartyMember { party_id, member_id }))
}
