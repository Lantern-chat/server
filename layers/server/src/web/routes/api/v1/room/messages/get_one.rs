use ftl::*;

use schema::Snowflake;

use super::ApiResponse;
use crate::{Authorization, ServerState};

#[async_recursion]
pub async fn get_one(
    route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
) -> ApiResponse {
    let msg = crate::backend::api::room::messages::get::get_one(route.state, auth, room_id, msg_id).await?;

    Ok(reply::json(msg).into_response())
}
