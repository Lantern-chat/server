use ftl::*;

use schema::Snowflake;

use super::ApiResponse;
use crate::{Authorization, ServerState};

pub async fn delete(
    route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
) -> ApiResponse {
    crate::backend::api::room::messages::delete::delete_msg(route.state, auth, room_id, msg_id).await?;

    Ok(StatusCode::OK.into_response())
}
