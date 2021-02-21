use http::{Method, StatusCode};

use crate::server::ftl::*;

pub mod create;

pub async fn party(mut route: Route) -> impl Reply {
    match route.next_segment_method() {
        (&Method::POST, End) => "create".into_response(),

        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
