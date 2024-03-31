use sdk::api::commands::user::UserLogout;

use super::*;

#[async_recursion]
pub async fn logout(route: Route<ServerState>, _auth: Authorization) -> ApiResult {
    Ok(Procedure::from(UserLogout {}))
}
