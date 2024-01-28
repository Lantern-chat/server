use sdk::api::commands::party::{GetPartyMember, GetPartyMembers};

use super::*;

#[async_recursion]
pub async fn get_members(route: Route<ServerState>, auth: Authorization, party_id: Snowflake) -> ApiResult {
    Ok(RawMessage::authorized(auth, GetPartyMembers { party_id }))
}

#[async_recursion]
pub async fn get_member(
    route: Route<ServerState>,
    auth: Authorization,
    party_id: Snowflake,
    member_id: Snowflake,
) -> ApiResult {
    Ok(RawMessage::authorized(auth, GetPartyMember { party_id, member_id }))
}
