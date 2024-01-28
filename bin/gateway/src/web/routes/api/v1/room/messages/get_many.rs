use sdk::api::commands::room::GetMessages;

use super::*;

#[async_recursion] #[rustfmt::skip]
pub async fn get_many(route: Route<ServerState>, auth: Authorization, room_id: Snowflake) -> ApiResult {
    Ok(RawMessage::authorized(auth, GetMessages {
        room_id,
        body: match route.query() {
            None => Default::default(),
            Some(form) => form?,
        },
    }))
}
