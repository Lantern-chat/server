use ftl::*;
use schema::Snowflake;

use super::ApiResponse;
use crate::{Authorization, ServerState};

#[async_recursion]
pub async fn get_profile(
    route: Route<ServerState>,
    auth: Authorization,
    member_id: Snowflake,
    party_id: Snowflake,
) -> ApiResponse {
    Ok(reply::json(
        crate::backend::api::user::profile::get_profile(route.state, auth, member_id, Some(party_id)).await?,
    )
    .into_response())
}

#[async_recursion]
pub async fn patch_profile(
    route: Route<ServerState>,
    auth: Authorization,
    party_id: Snowflake,
) -> ApiResponse {
    unimplemented!()
}
