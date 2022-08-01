use ftl::*;
use sdk::Snowflake;

use super::ApiResponse;
use crate::{Authorization, Error, ServerState};

pub async fn profile(route: Route<ServerState>, auth: Authorization, user_id: Snowflake) -> ApiResponse {
    let profile = crate::backend::api::user::profile::get_profile(route.state, auth, user_id, None).await?;

    Ok(reply::json(profile).into_response())
}
