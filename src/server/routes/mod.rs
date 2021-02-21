use http::{Method, Response, StatusCode};
use hyper::Body;

pub use super::ftl::{fs, real_ip, Reply, Route};

pub mod api;

pub async fn routes(mut route: Route) -> Response<Body> {
    if cfg!(debug_assertions) {
        log::info!(
            "{:?}: {} {}",
            real_ip::get_real_ip(&route),
            route.req.method(),
            route.req.uri()
        )
    }

    match route.next_segment_method() {
        (_, "api") => api::api(route).await,

        (&Method::GET, "static") => fs::dir(&route, "frontend/dist").await.into_response(),

        (&Method::GET, _) => fs::file(&route, "frontend/dist/index.html")
            .await
            .into_response(),

        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
