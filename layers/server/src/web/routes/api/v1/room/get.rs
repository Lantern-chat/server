use ftl::*;

use schema::Snowflake;

use super::ApiResponse;
use crate::Authorization;

pub async fn get_room(
    route: Route<crate::ServerState>,
    auth: Authorization,
    room_id: Snowflake,
) -> ApiResponse {
    let room =
        crate::backend::api::room::get::get_room(route.state, auth, room_id)
            .await?;

    Ok(reply::json(room).into_response())
}
