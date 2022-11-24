use ftl::*;

use super::ApiResponse;
use crate::{Authorization, ServerState};

#[async_recursion]
pub async fn post(
    mut route: Route<ServerState>,
    auth: Authorization,
) -> ApiResponse {
    let form = body::any(&mut route).await?;

    let party = crate::backend::api::party::create::create_party(
        route.state,
        auth,
        form,
    )
    .await?;

    Ok(reply::json(party).into_response())
}
