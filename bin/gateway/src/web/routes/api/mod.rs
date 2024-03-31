pub mod v1;

use ftl::{body, End, Exact, Method, Reply, Response, Route, StatusCode};

use async_recursion::async_recursion;

#[rustfmt::skip]
pub async fn api(mut route: Route<crate::state::ServerState>) -> Response {
    match route.next().segment() {
        // ANY /api/v1
        Exact("v1") => v1::api_v1(route).await,
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
