use super::*;

#[async_recursion]
pub async fn pin_message(
    route: Route<ServerState>,
    auth: Authorization,
    msg_id: MessageId,
    pin_id: FolderId,
) -> ApiResult {
    unimplemented!()
}

#[async_recursion]
pub async fn unpin_message(
    route: Route<ServerState>,
    auth: Authorization,
    msg_id: MessageId,
    pin_id: FolderId,
) -> ApiResult {
    unimplemented!()
}

#[async_recursion]
pub async fn star_message(route: Route<ServerState>, auth: Authorization, msg_id: MessageId) -> ApiResult {
    unimplemented!()
}

#[async_recursion]
pub async fn unstar_message(route: Route<ServerState>, auth: Authorization, msg_id: MessageId) -> ApiResult {
    unimplemented!()
}
