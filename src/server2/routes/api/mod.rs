use http::Response;
use hyper::Body;

pub use super::Route;

pub mod v1;

pub async fn api(mut route: Route) -> Response<Body> {
    match route.next_segment() {
        "v1" => v1::api_v1(route).await,
        _ => Response::new(Body::from("404 Not Found")),
    }
}