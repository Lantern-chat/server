use http::Response;
use hyper::Body;

pub use super::Route;

pub async fn api_v1(mut route: Route) -> Response<Body> {
    match route.next_segment() {
        "users" => Response::new(Body::from("Users")),
        _ => Response::new(Body::from("404 Not Found")),
    }
}