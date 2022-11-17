use ftl::*;
use futures::FutureExt;
use sdk::Snowflake;

use super::ApiResponse;
use crate::{web::auth::authorize, Error};

pub mod account;
pub mod billing;
pub mod login;
pub mod logout;
pub mod prefs;
pub mod profile;
pub mod sessions;

pub mod friends {
    use super::ApiResponse;

    pub mod get;
}

#[rustfmt::skip]
pub async fn me(mut route: Route<crate::ServerState>) -> ApiResponse {
    match route.next().method_segment() {
        // POST /api/v1/user/@me
        (&Method::POST, End) => login::login(route).await,

        // Everything else requires authorization
        _ => {
            let auth = authorize(&route).await?;

            match route.method_segment() {
                (&Method::DELETE, End) => logout::logout(route, auth).await,
                (&Method::GET, Exact("sessions")) => sessions::sessions(route, auth).await,
                (&Method::PATCH, Exact("prefs")) => prefs::prefs(route, auth).await,
                (&Method::PATCH, Exact("account")) => account::patch_account(route, auth).await,
                (&Method::PATCH, Exact("profile")) => profile::patch_profile(route, auth).await,
                (_, Exact("friends")) => match route.next().method_segment() {
                    (&Method::GET, End) => friends::get::friends(route, auth).await,
                    (_, Exact(_)) => {
                        let Some(Ok(user_id)) = route.param::<Snowflake>() else {
                            return Err(Error::BadRequest);
                        };

                        match route.method() {
                            &Method::POST => todo!("AddFriend"),
                            &Method::DELETE => todo!("RemoveFriend"),
                            &Method::PATCH => todo!("PatchFriend"),
                            _ => Err(Error::MethodNotAllowed)
                        }
                    }
                    _ => Err(Error::NotFound),
                },
                _ => Err(Error::NotFound),
            }
        },
    }
}
