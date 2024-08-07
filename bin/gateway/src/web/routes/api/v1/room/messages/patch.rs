use sdk::api::commands::room::EditMessage;

use super::*;

#[async_recursion] #[rustfmt::skip]
pub async fn patch(mut route: Route<ServerState>, _auth: Authorization, room_id: RoomId, msg_id: MessageId) -> ApiResult {
    Ok(Procedure::from(EditMessage {
        room_id,
        msg_id,
        body: body::any(&mut route).await?,
    }))
}
