use ftl::*;
use sdk::Snowflake;

use super::ApiResponse;
use crate::{Authorization, Error, ServerState};

#[async_recursion]
pub async fn get(route: Route<ServerState>, auth: Authorization, user_id: Snowflake) -> ApiResponse {
    let user = crate::backend::api::user::get::get_full_user(route.state, auth, user_id).await?;

    Ok(reply::json(user).into_response())
}
