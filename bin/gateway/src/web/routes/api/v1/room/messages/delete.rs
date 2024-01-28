use sdk::api::commands::room::DeleteMessage;

use super::*;

#[async_recursion]
pub async fn delete(
    route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
) -> ApiResult {
    Ok(RawMessage::authorized(auth, DeleteMessage { room_id, msg_id }))
}
