use ftl::*;

use super::ApiResponse;
use crate::{Authorization, ServerState};

use sdk::api::commands::party::CreatePartyInviteBody;

#[async_recursion]
pub async fn post(
    mut route: Route<ServerState>,
    auth: Authorization,
) -> ApiResponse {
    let form = body::any::<CreatePartyInviteBody, _>(&mut route).await?;

    Ok(().into_response())
}
