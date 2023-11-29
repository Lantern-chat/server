use super::*;

#[async_recursion]
pub async fn patch(
    mut route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
) -> WebResult {
    let form = body::any(&mut route).await?;

    let msg =
        crate::backend::api::room::messages::edit::edit_message(route.state, auth, room_id, msg_id, form).await?;

    Ok(match msg {
        Some(msg) => WebResponse::new(msg),
        None => StatusCode::OK.into(),
    })
}
