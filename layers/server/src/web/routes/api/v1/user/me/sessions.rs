use ftl::*;

use super::ApiResponse;
use crate::{Authorization, ServerState};

#[async_recursion]
pub async fn sessions(
    route: Route<ServerState>,
    auth: Authorization,
) -> ApiResponse {
    let sessions = crate::backend::api::user::me::sessions::list_sessions(
        route.state,
        auth,
    )
    .await?;

    Ok(reply::json::array_stream(sessions).into_response())
}
