use ftl::*;

use crate::routes::api::auth::{self, AuthError, AuthToken};

pub async fn check(route: Route<crate::ServerState>) -> impl Reply {
    match auth::authorize(&route).await {
        Ok(auth) => StatusCode::OK.into_response(),
        Err(e) => e.into_response(),
    }
}
