use http::{Response, StatusCode};
use hyper::Body;

pub use super::{Reply, Route};

pub mod util;

pub mod v1;

pub async fn api(mut route: Route) -> Response<Body> {
    match route.next_segment() {
        "v1" => v1::api_v1(route).await,

        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
