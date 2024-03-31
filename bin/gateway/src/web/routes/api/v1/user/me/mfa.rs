use sdk::api::commands::user::{Confirm2FA, Enable2FA, Remove2FA};

use super::*;

#[async_recursion] #[rustfmt::skip]
pub async fn post_2fa(mut route: Route<ServerState>, auth: Authorization) -> ApiResult {
    Ok(Procedure::from(Enable2FA { body: body::any(&mut route).await? }))
}

#[async_recursion] #[rustfmt::skip]
pub async fn patch_2fa(mut route: Route<ServerState>, auth: Authorization) -> ApiResult {
    Ok(Procedure::from(Confirm2FA { body: body::any(&mut route).await? }))
}

#[async_recursion] #[rustfmt::skip]
pub async fn delete_2fa(mut route: Route<ServerState>, auth: Authorization) -> ApiResult {
    Ok(Procedure::from(Remove2FA { body: body::any(&mut route).await? }))
}
