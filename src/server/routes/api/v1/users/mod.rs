use http::{Method, StatusCode};

use crate::server::ftl::*;

pub mod check;
pub mod login;
pub mod logout;
pub mod register;

pub async fn users(mut route: Route) -> impl Reply {
    match route.next_segment_method() {
        // POST /api/v1/users
        (&Method::POST, End) => register::register(route).await.into_response(),

        // POST /api/v1/users/login
        (&Method::POST, Exact("login")) => login::login(route).await.into_response(),

        // DELETE /api/v1/users/logout
        (&Method::DELETE, Exact("logout")) => logout::logout(route).await.into_response(),

        // HEAD /api/v1/users/check
        (&Method::HEAD, Exact("check")) => check::check(route).await.into_response(),

        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
