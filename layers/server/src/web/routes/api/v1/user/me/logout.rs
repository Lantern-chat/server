use crate::{Authorization, Error, ServerState};

use ftl::*;

pub async fn logout(route: Route<ServerState>, auth: Authorization) -> Result<Response> {
    if let Err(e) = crate::backend::api::user::me::logout::logout_user(&route.state, auth).await {
        log::error!("Logout error: {e}");
    }

    Ok(StatusCode::NO_CONTENT.into_response())
}
