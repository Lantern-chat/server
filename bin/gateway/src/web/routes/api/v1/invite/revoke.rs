use super::*;

#[async_recursion]
pub async fn revoke(route: Route<ServerState>, auth: Authorization, code: SmolStr) -> ApiResult {
    err(CommonError::Unimplemented)
}
