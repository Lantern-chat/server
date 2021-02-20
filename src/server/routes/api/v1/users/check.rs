use http::StatusCode;

use crate::server::auth::{self, AuthError, AuthToken};

use super::{Reply, Route};

pub async fn check(mut route: Route) -> impl Reply {
    match auth::authorize(&route).await {
        Ok(auth) => StatusCode::OK.into_response(),
        Err(e) => e.into_response(),
    }
}
