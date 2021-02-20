use http::{Method, Response, StatusCode};
use hyper::Body;

pub use super::{Reply, Route};

pub mod check;
pub mod login;
pub mod logout;
pub mod register;

pub async fn users(mut route: Route) -> Response<Body> {
    match (route.req.method().clone(), route.next_segment()) {
        // POST /api/v1/users
        (Method::POST, "") => register::register(route).await.into_response(),

        // POST /api/v1/users/login
        (Method::POST, "login") => login::login(route).await.into_response(),

        // DELETE /api/v1/users/logout
        (Method::DELETE, "logout") => logout::logout(route).await.into_response(),

        // GET /api/v1/users/check
        (Method::HEAD, "check") => check::check(route).await.into_response(),

        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
