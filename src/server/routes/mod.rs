use http::{Response, StatusCode};
use hyper::Body;

pub use super::reply::Reply;
pub use super::service::Route;

use super::{
    fs::{dir, file},
    util::get_real_ip,
};

pub mod api;

pub async fn routes(mut route: Route) -> Response<Body> {
    if cfg!(debug_assertions) {
        log::info!(
            "{:?}: {} {}",
            get_real_ip(&route),
            route.req.method(),
            route.req.uri()
        )
    }

    match route.next_segment() {
        "api" => api::api(route).await,

        "static" => dir(&route, "frontend/dist").await.into_response(),

        _ => file(&route, "frontend/dist/index.html")
            .await
            .into_response(),
    }
}
