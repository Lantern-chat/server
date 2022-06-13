use ftl::*;

use crate::{Error, web::auth::authorize};

pub mod avatar;
pub mod friends;
pub mod login;
pub mod logout;
pub mod prefs;
pub mod sessions;
pub mod account;
pub mod billing;

pub async fn me(mut route: Route<crate::ServerState>) -> Result<Response, Error> {
    match route.next().method_segment() {
        // POST /api/v1/user/@me
        (&Method::POST, End) => login::login(route).await,

        // Everything else requires authorization
        _ => {
            let auth = authorize(&route).await?;

            match route.method_segment() {
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
