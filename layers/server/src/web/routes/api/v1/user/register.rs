use ftl::*;

use super::ApiResponse;
use crate::ServerState;

#[async_recursion]
pub async fn register(mut route: Route<ServerState>) -> ApiResponse {
    let form = body::any(&mut route).await?;

    let session = crate::backend::api::user::register::register_user(
        route.state,
        route.real_addr,
        form,
    )
    .await?;

    Ok(reply::json(session)
        .with_status(StatusCode::CREATED)
        .into_response())
}
