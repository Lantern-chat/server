use sdk::api::commands::room::CreateMessage;

use super::*;

#[async_recursion]
pub async fn post(mut route: Route<ServerState>, _auth: Authorization, room_id: Snowflake) -> ApiResult {
    Ok(Procedure::from(CreateMessage {
        room_id,
        body: body::any(&mut route).await?,
    }))
}
