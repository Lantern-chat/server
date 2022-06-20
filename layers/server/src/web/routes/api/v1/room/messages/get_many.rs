use ftl::*;

use schema::Snowflake;

use super::ApiResponse;
use crate::Authorization;

pub async fn get_many(
    route: Route<crate::ServerState>,
    auth: Authorization,
    room_id: Snowflake,
) -> ApiResponse {
    let form = match route.query() {
        None => Default::default(),
        Some(form) => form?,
    };

    let msgs =
        crate::backend::api::room::messages::get_many::get_many(route.state, auth, room_id, form).await?;

    Ok(reply::json::array_stream(msgs).into_response())
}
