use ftl::*;

use crate::{web::auth::Authorization, ServerState};

pub async fn post(route: Route<ServerState>, auth: Authorization) -> Response {
    ().into_response()
}
