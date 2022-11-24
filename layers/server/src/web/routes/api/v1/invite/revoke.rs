use ftl::*;
use smol_str::SmolStr;

use crate::{Authorization, ServerState, Error};
use super::ApiResponse;

#[async_recursion]
pub async fn revoke(route: Route<ServerState>, auth: Authorization, code: SmolStr) -> ApiResponse {
    Err(Error::Unimplemented)
}
