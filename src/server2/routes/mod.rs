use http::Response;
use hyper::Body;

pub use super::reply::Reply;
pub use super::service::Route;

pub mod api;

pub async fn routes(mut route: Route) -> Response<Body> {
    match route.next_segment() {
        "api" => api::api(route).await,
        "" => Response::new(Body::from("Index")),
        _ => Response::new(Body::from("404 Not Found")),
    }
}
