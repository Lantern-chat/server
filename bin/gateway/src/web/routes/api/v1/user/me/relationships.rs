use super::*;

#[async_recursion]
pub async fn get_relationships(state: ServerState, auth: Authorization) -> WebResult {
    Ok(WebResponse::stream(
        crate::backend::api::user::me::relationships::get::get_relationships(state, auth).await?,
    ))
}
