use http::{Method, Response, StatusCode};
use hyper::Body;

use crate::server::ftl::*;

pub mod build;
pub mod party;
pub mod users;

pub async fn api_v1(mut route: Route) -> impl Reply {
    match route.next_segment_method() {
        (_, Exact("users")) => users::users(route).await.into_response(),

        (&Method::GET, Exact("build")) => build::build().into_response(),

        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
