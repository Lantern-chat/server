use super::*;

#[async_recursion]
pub async fn sessions(route: Route<ServerState>, auth: Authorization) -> WebResult {
    Ok(WebResponse::stream(
        crate::backend::api::user::me::sessions::list_sessions(route.state, auth).await?,
    ))
}
