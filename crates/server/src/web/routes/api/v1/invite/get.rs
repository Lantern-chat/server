use ftl::*;

use crate::{web::auth::Authorization, ServerState};

pub async fn get_invite(route: Route<ServerState>, auth: Authorization, code: String) -> Response {
    ().into_response()
}
