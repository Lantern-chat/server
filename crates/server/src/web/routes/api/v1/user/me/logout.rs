use crate::ctrl::{auth, user::me::logout::logout_user};
use crate::ServerState;

use ftl::*;

pub async fn logout(route: Route<ServerState>, auth: auth::Authorization) -> impl Reply {
    if let Err(e) = logout_user(route.state, auth).await {
        log::error!("Logout error: {}", e);
    }

    StatusCode::OK.into_response()
}
