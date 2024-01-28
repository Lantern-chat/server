use sdk::api::commands::user::GetSessions;

use super::*;

#[async_recursion]
pub async fn sessions(route: Route<ServerState>, auth: Authorization) -> ApiResult {
    Ok(RawMessage::authorized(auth, GetSessions {}))
}
