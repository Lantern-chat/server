use sdk::api::commands::user::UpdateUserPrefs;

use super::*;

#[async_recursion] #[rustfmt::skip]
pub async fn prefs(mut route: Route<ServerState>, auth: Authorization) -> ApiResult {
    Ok(RawMessage::authorized(auth, UpdateUserPrefs {
        body: body::any(&mut route).await?,
    }))
}
