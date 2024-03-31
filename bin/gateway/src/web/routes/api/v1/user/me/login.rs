use sdk::api::commands::user::UserLogin;

use super::*;

#[async_recursion]
pub async fn login(mut route: Route<ServerState>) -> ApiResult {
    Ok(Procedure::from(UserLogin {
        body: body::any(&mut route).await?,
    }))
}
