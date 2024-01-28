use super::*;

#[async_recursion]
pub async fn patch_account(mut route: Route<ServerState>, auth: Authorization) -> ApiResult {
    err(CommonError::Unimplemented)

    // body::any(&mut route).await?
    //Ok(RawMessage::authorized(auth, ))
}
