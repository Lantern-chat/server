use sdk::api::commands::user::GetSessions;

use super::*;

#[async_recursion]
pub async fn sessions(route: Route<ServerState>, _auth: Authorization) -> ApiResult {
    Ok(Procedure::from(GetSessions {}))
}
