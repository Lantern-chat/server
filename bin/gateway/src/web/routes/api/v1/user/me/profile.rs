use sdk::api::commands::user::UpdateUserProfile;

use super::*;

#[async_recursion] #[rustfmt::skip]
pub async fn patch_profile(mut route: Route<ServerState>, auth: Authorization) -> ApiResult {
    Ok(RawMessage::authorized(auth, UpdateUserProfile {
        body: body::any(&mut route).await?,
    }))
}
