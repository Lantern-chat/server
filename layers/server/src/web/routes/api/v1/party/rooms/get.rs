use crate::backend::api::party::rooms::get::{get_rooms, RoomScope};

use super::*;

#[async_recursion]
pub async fn get(route: Route<crate::ServerState>, auth: Authorization, party_id: Snowflake) -> WebResult {
    Ok(WebResponse::stream(
        get_rooms(route.state, auth, RoomScope::Party(party_id)).await?,
    ))
}
