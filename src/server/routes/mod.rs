use http::{Response, StatusCode};
use hyper::Body;

pub use super::reply::Reply;
pub use super::service::Route;
use super::{
    fs::{dir, file},
    util::get_real_ip,
};

pub mod api;

const INDEX_PATH: &'static str = "frontend/dist/index.html";
const STATIC_PATH: &'static str = "frontend/dist";

pub async fn routes(mut route: Route) -> Response<Body> {
    if cfg!(debug_assertions) {
        let ip = get_real_ip(&route);

        log::info!("{:?}: {} {}", ip, route.req.method(), route.req.uri())
    }

    match route.next_segment() {
        "api" => api::api(route).await,

        "static" => dir(&route, STATIC_PATH).await.into_response(),

        _ => file(&route, INDEX_PATH).await.into_response(),
    }
}
