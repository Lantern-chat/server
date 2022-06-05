use ftl::*;

use crate::web::{auth::authorize, routes::api::ApiError};

pub mod avatar;
pub mod friends;
pub mod login;
pub mod logout;
pub mod prefs;
pub mod sessions;
pub mod account;
pub mod billing;

pub async fn me(mut route: Route<crate::ServerState>) -> Response {
    match route.next().method_segment() {
        // POST /api/v1/user/@me
        (&Method::POST, End) => login::login(route).await,

        // Everything else requires authorization
        _ => match authorize(&route).await {
            Err(e) => ApiError::err(e).into_response(),
            Ok(auth) => match route.method_segment() {
                (&Method::DELETE, End) => logout::logout(route, auth).await,
                (&Method::GET, Exact("sessions")) => sessions::sessions(route, auth).await,
                (&Method::GET, Exact("friends")) => friends::friends(route, auth).await,
                (&Method::POST, Exact("avatar")) => avatar::post_avatar(route, auth).await,
                (&Method::PATCH, Exact("prefs")) => prefs::prefs(route, auth).await,
                (&Method::PATCH, Exact("account")) => account::patch_account(route, auth).await,
                _ => ApiError::not_found().into_response(),
            },
        },
    }
}