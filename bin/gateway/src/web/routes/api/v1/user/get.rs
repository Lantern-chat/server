use sdk::api::commands::user::GetUser;

use super::*;

#[async_recursion]
pub async fn get(route: Route<ServerState>, _auth: Authorization, user_id: Snowflake) -> ApiResult {
    Ok(Procedure::from(GetUser { user_id }))
}
