use sdk::api::commands::invite::RedeemInvite;

use super::*;

#[async_recursion] #[rustfmt::skip]
pub async fn redeem(mut route: Route<ServerState>, auth: Authorization, code: SmolStr) -> ApiResult {
    Ok(RawMessage::authorized(auth, RedeemInvite { code, body: body::any(&mut route).await? }))
}
