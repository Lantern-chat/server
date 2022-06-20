use ftl::*;

use super::ApiResponse;
use crate::ServerState;

pub async fn login(mut route: Route<ServerState>) -> ApiResponse {
    let form = body::any(&mut route).await?;

    let session = crate::backend::api::user::me::login::login(
        route.state,
        route.real_addr,
        form,
    )
    .await?;

    Ok(reply::json(session)
        .with_status(StatusCode::CREATED)
        .into_response())
}
