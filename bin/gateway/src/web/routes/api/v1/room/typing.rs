use sdk::api::commands::room::StartTyping;

use super::*;

#[async_recursion] #[rustfmt::skip]
pub async fn trigger_typing(mut route: Route<ServerState>, _auth: Authorization, room_id: Snowflake) -> ApiResult {
    Ok(Procedure::from(StartTyping {
        room_id,
        body: body::any(&mut route).await.unwrap_or_default()
    }))
}
