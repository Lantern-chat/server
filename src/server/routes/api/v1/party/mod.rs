use http::{Method, StatusCode};

pub use super::{auth, ApiError, Reply, Route};

pub mod create;

pub async fn party(mut route: Route) -> impl Reply {
    match route.next_segment_method() {
        (&Method::POST, "") => "create".into_response(),

        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
