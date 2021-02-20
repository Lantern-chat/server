use http::{Response, StatusCode};
use hyper::Body;

pub use super::{Reply, Route};

pub mod users;

pub async fn api_v1(mut route: Route) -> Response<Body> {
    match route.next_segment() {
        "users" => users::users(route).await,

        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
