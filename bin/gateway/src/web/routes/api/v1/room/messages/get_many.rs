use sdk::api::commands::room::GetMessages;

use super::*;

#[async_recursion]
pub async fn get_many(route: Route<ServerState>, _auth: Authorization, room_id: RoomId) -> ApiResult {
    Ok(Procedure::from(GetMessages {
        room_id,
        body: match route.query() {
            None => Default::default(),
            Some(form) => form?,
        },
    }))
}
