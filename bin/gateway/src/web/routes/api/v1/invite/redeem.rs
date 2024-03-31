use sdk::api::commands::invite::RedeemInvite;

use super::*;

#[async_recursion] #[rustfmt::skip]
pub async fn redeem(mut route: Route<ServerState>, _auth: Authorization, code: SmolStr) -> ApiResult {
    Ok(Procedure::from(RedeemInvite { code, body: body::any(&mut route).await? }))
}
