use sdk::api::commands::user::UserLogout;

use super::*;

#[async_recursion] #[rustfmt::skip]
pub async fn logout(route: Route<ServerState>, auth: Authorization) -> ApiResult {
    Ok(RawMessage::authorized(auth, UserLogout {}))
}
