use http::StatusCode;

use crate::server::routes::api::auth::{self, AuthError, AuthToken};

use crate::server::ftl::*;

pub async fn check(mut route: Route) -> impl Reply {
    match auth::authorize(&route).await {
        Ok(auth) => StatusCode::OK.into_response(),
        Err(e) => e.into_response(),
    }
}
