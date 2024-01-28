use sdk::api::commands::user::{Confirm2FA, Enable2FA, Remove2FA};

use super::*;

#[async_recursion] #[rustfmt::skip]
pub async fn post_2fa(mut route: Route<ServerState>, auth: Authorization) -> ApiResult {
    Ok(RawMessage::authorized(auth, Enable2FA { body: body::any(&mut route).await? }))
}

#[async_recursion] #[rustfmt::skip]
pub async fn patch_2fa(mut route: Route<ServerState>, auth: Authorization) -> ApiResult {
    Ok(RawMessage::authorized(auth, Confirm2FA { body: body::any(&mut route).await? }))
}

#[async_recursion] #[rustfmt::skip]
pub async fn delete_2fa(mut route: Route<ServerState>, auth: Authorization) -> ApiResult {
    Ok(RawMessage::authorized(auth, Remove2FA { body: body::any(&mut route).await? }))
}
