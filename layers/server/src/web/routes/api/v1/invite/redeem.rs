use ftl::*;
use smol_str::SmolStr;

use super::ApiResponse;
use crate::{Authorization, ServerState};

#[async_recursion]
pub async fn redeem(mut route: Route<ServerState>, auth: Authorization, code: SmolStr) -> ApiResponse {
    let form = body::any(&mut route).await?;

    let res = crate::backend::api::invite::redeem::redeem_invite(route.state, auth, code, form).await?;

    Ok(().into_response())
}
