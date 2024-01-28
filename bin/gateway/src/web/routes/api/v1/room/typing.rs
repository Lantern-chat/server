use sdk::api::commands::room::StartTyping;

use super::*;

#[async_recursion] #[rustfmt::skip]
pub async fn trigger_typing(mut route: Route<ServerState>, auth: Authorization, room_id: Snowflake) -> ApiResult {
    Ok(RawMessage::authorized(auth, StartTyping {
        room_id,
        body: body::any(&mut route).await.unwrap_or_default()
    }))
}
