use sdk::api::commands::room::DeleteMessage;

use super::*;

#[async_recursion]
pub async fn delete(
    route: Route<ServerState>,
    _auth: Authorization,
    room_id: RoomId,
    msg_id: MessageId,
) -> ApiResult {
    Ok(Procedure::from(DeleteMessage { room_id, msg_id }))
}
