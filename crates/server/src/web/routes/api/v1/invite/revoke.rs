use ftl::*;

use crate::{web::auth::Authorization, ServerState};

pub async fn revoke(route: Route<ServerState>, auth: Authorization, code: String) -> Response {
    ().into_response()
}
