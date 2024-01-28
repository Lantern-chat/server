use sdk::api::commands::room::EditMessage;

use super::*;

#[async_recursion] #[rustfmt::skip]
pub async fn patch(mut route: Route<ServerState>, auth: Authorization, room_id: Snowflake, msg_id: Snowflake) -> ApiResult {
    Ok(RawMessage::authorized(auth, EditMessage {
        room_id,
        msg_id,
        body: body::any(&mut route).await?,
    }))
}
