use ftl::*;
use schema::Snowflake;

use super::ApiResponse;
use crate::{Authorization, ServerState};

#[async_recursion]
pub async fn get_members(route: Route<ServerState>, auth: Authorization, party_id: Snowflake) -> ApiResponse {
    let members =
        crate::backend::api::party::members::get_members(route.state, party_id, auth.user_id).await?;

    Ok(reply::json::array_stream(members).into_response())
}
