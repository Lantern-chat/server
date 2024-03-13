use super::*;

#[async_recursion]
pub async fn patch_account(mut route: Route<ServerState>, auth: Authorization) -> ApiResult {
    Err(Error::Unimplemented)

    // body::any(&mut route).await?
    //Ok(RawMessage::authorized(auth, ))
}
