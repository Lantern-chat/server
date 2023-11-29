use super::*;

#[async_recursion]
pub async fn pin_message(
    route: Route<ServerState>,
    auth: Authorization,
    msg_id: Snowflake,
    pin_id: Snowflake,
) -> WebResult {
    unimplemented!()
}

#[async_recursion]
pub async fn unpin_message(
    route: Route<ServerState>,
    auth: Authorization,
    msg_id: Snowflake,
    pin_id: Snowflake,
) -> WebResult {
    unimplemented!()
}

#[async_recursion]
pub async fn star_message(route: Route<ServerState>, auth: Authorization, msg_id: Snowflake) -> WebResult {
    unimplemented!()
}

#[async_recursion]
pub async fn unstar_message(route: Route<ServerState>, auth: Authorization, msg_id: Snowflake) -> WebResult {
    unimplemented!()
}
