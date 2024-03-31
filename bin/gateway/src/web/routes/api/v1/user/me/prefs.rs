use sdk::api::commands::user::UpdateUserPrefs;

use super::*;

#[async_recursion]
pub async fn prefs(mut route: Route<ServerState>, _auth: Authorization) -> ApiResult {
    Ok(Procedure::from(UpdateUserPrefs {
        body: body::any(&mut route).await?,
    }))
}
