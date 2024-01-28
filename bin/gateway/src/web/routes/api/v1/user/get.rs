use sdk::api::commands::user::GetUser;

use super::*;

#[async_recursion]
pub async fn get(route: Route<ServerState>, auth: Authorization, user_id: Snowflake) -> ApiResult {
    Ok(RawMessage::authorized(auth, GetUser { user_id }))
}
