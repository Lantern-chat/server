use crate::ctrl::{auth, user::me::logout::logout_user};
use crate::ServerState;

use ftl::*;

pub async fn logout(route: Route<ServerState>, auth: auth::Authorization) -> Response {
    if let Err(e) = logout_user(route.state, auth).await {
        log::error!("Logout error: {e}");
    }

    StatusCode::NO_CONTENT.into_response()
}