use http::{Response, StatusCode};
use hyper::Body;

pub use super::{ApiError, Reply, Route, auth};

pub mod build;
pub mod users;

pub async fn api_v1(mut route: Route) -> impl Reply {
    match route.next_segment() {
        "users" => users::users(route).await.into_response(),
        "build" => build::build().into_response(),

        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
