use http::{Method, Response, StatusCode};
use hyper::Body;

use crate::server::ftl::*;

pub mod api;

pub async fn routes(mut route: Route) -> Response<Body> {
    match route.next_segment_method() {
        (_, Exact("api")) => api::api(route).await,

        (&Method::GET, Exact("static")) => fs::dir(&route, "frontend/dist").await.into_response(),

        (&Method::GET, _) => fs::file(&route, "frontend/dist/index.html")
            .await
            .into_response(),

        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
