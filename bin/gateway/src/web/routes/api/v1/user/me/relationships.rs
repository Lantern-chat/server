use sdk::api::commands::user::GetRelationships;

use super::*;

#[async_recursion]
pub async fn get_relationships(state: ServerState, auth: Authorization) -> ApiResult {
    Ok(RawMessage::authorized(auth, GetRelationships {}))
}
