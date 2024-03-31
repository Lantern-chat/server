use sdk::api::commands::user::GetRelationships;

use super::*;

#[async_recursion]
pub async fn get_relationships(state: ServerState, _auth: Authorization) -> ApiResult {
    Ok(Procedure::from(GetRelationships {}))
}
