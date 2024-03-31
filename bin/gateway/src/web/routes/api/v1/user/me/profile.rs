use sdk::api::commands::user::UpdateUserProfile;

use super::*;

#[async_recursion]
pub async fn patch_profile(mut route: Route<ServerState>, _auth: Authorization) -> ApiResult {
    Ok(Procedure::from(UpdateUserProfile {
        body: body::any(&mut route).await?,
    }))
}
