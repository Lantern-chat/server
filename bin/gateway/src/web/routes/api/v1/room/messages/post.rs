use sdk::api::commands::room::CreateMessage;

use super::*;

#[async_recursion] #[rustfmt::skip]
pub async fn post(mut route: Route<ServerState>, auth: Authorization, room_id: Snowflake) -> ApiResult {
    Ok(RawMessage::authorized(auth, CreateMessage {
        room_id,
        body: body::any(&mut route).await?,
    }))
}
