use http::StatusCode;

pub mod auth;
pub mod error;
pub mod util;

pub mod v1;

use crate::server::ftl::*;

pub async fn api(mut route: Route) -> Response {
    match route.next().segment() {
        Exact("v1") => v1::api_v1(route).await.into_response(),

        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
