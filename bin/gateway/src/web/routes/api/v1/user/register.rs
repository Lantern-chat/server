use sdk::api::commands::user::UserRegister;

use super::*;

#[async_recursion]
pub async fn register(mut route: Route<ServerState>) -> ApiResult {
    Ok(Procedure::from(UserRegister {
        body: body::any(&mut route).await?,
    }))
}
