use super::*;

#[async_recursion]
pub async fn get_invite(route: Route<ServerState>, auth: Authorization, code: SmolStr) -> WebResult {
    Err(Error::Unimplemented)
}
