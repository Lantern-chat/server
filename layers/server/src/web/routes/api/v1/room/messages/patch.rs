use ftl::*;

use schema::Snowflake;

use super::ApiResponse;
use crate::{Authorization, ServerState};

pub async fn patch(
    mut route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
) -> ApiResponse {
    let form = body::any(&mut route).await?;

    let msg =
        crate::backend::api::room::messages::edit::edit_message(route.state, auth, room_id, msg_id, form)
            .await?;

    Ok(match msg {
        Some(ref msg) => reply::json(msg).into_response(),
        None => StatusCode::OK.into_response(),
    })
}
