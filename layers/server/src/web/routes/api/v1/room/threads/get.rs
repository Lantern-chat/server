use ftl::*;
use schema::Snowflake;

use crate::ServerState;

use super::ApiResponse;
use crate::Authorization;

pub async fn get(
    route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    thread_id: Snowflake,
) -> ApiResponse {
    let thread = crate::backend::api::room::threads::get::get_thread(
        route.state,
        auth,
        room_id,
        thread_id,
    )
    .await?;

    Ok(reply::json(thread).into_response())
}
