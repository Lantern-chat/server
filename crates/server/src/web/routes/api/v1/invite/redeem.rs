use ftl::*;
use smol_str::SmolStr;

use crate::{web::auth::Authorization, ServerState};

pub async fn redeem(route: Route<ServerState>, auth: Authorization, code: SmolStr) -> Response {
    ().into_response()
}
