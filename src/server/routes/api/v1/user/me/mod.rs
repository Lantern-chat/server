use http::{Method, StatusCode};

use crate::{
    db::Snowflake,
    server::{ftl::*, routes::api::auth::authorize},
};

pub mod login;
pub mod logout;
pub mod sessions;

pub async fn me(mut route: Route) -> impl Reply {
    match route.next().method_segment() {
        // POST /api/v1/user/@me
        (&Method::POST, End) => login::login(route).await.into_response(),

        // Everything else requires authorization
        _ => match authorize(&route).await {
            Err(e) => e.into_response(),
            Ok(auth) => match route.method_segment() {
                (&Method::DELETE, End) => logout::logout(route, auth).await.into_response(),
                (&Method::GET, Exact("sessions")) => "".into_response(),
                _ => StatusCode::NOT_FOUND.into_response(),
            },
        },
    }
}
