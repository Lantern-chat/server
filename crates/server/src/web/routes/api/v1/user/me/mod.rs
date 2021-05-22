use ftl::*;

use crate::web::{auth::authorize, routes::api::ApiError};

pub mod login;
pub mod logout;
pub mod sessions;

pub async fn me(mut route: Route<crate::ServerState>) -> impl Reply {
    match route.next().method_segment() {
        // POST /api/v1/user/@me
        (&Method::POST, End) => login::login(route).await.into_response(),

        // Everything else requires authorization
        _ => match authorize(&route).await {
            Err(e) => ApiError::err(e).into_response(),
            Ok(auth) => match route.method_segment() {
                (&Method::DELETE, End) => logout::logout(route, auth).await.into_response(),
                (&Method::GET, Exact("sessions")) => {
                    sessions::sessions(route, auth).await.into_response()
                }
                _ => ApiError::not_found().into_response(),
            },
        },
    }
}
