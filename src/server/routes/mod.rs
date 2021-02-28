use http::{Method, StatusCode};

use crate::server::ftl::*;

pub mod api;

pub async fn entry(mut route: Route) -> Response {
    if let Err(_) = route.apply_method_override() {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }

    match route.next().method_segment() {
        (_, Exact("api")) => api::api(route).await,

        (&Method::GET, Exact("static")) | (&Method::HEAD, Exact("static")) => {
            fs::dir(&route, "frontend/dist").await.into_response()
        }

        (&Method::GET, _) | (&Method::HEAD, _) => fs::file(&route, "frontend/dist/index.html")
            .await
            .into_response(),

        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
