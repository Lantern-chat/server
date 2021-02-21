use http::{Method, Response, StatusCode};
use hyper::Body;

pub use super::{auth, ApiError, Reply, Route};

pub mod build;
pub mod party;
pub mod users;

pub async fn api_v1(mut route: Route) -> impl Reply {
    match route.next_segment_method() {
        (_, "users") => users::users(route).await.into_response(),

        (&Method::GET, "build") => build::build().into_response(),

        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
