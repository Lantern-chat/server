use http::{Response, StatusCode};
use hyper::Body;

pub use self::error::ApiError;
pub use super::{Reply, Route};

pub mod auth;
pub mod error;
pub mod util;

pub mod v1;

pub async fn api(mut route: Route) -> Response<Body> {
    match route.next_segment() {
        "v1" => v1::api_v1(route).await.into_response(),

        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
