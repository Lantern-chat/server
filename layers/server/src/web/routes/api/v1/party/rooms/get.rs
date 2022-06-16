use ftl::*;

use schema::Snowflake;

use super::ApiResponse;
use crate::Authorization;

pub async fn get(
    route: Route<crate::ServerState>,
    auth: Authorization,
    party_id: Snowflake,
) -> ApiResponse {
    let rooms = crate::backend::api::party::rooms::get::get_rooms(
        route.state,
        auth,
        party_id,
    )
    .await?;

    Ok(reply::json(rooms).into_response())
}
