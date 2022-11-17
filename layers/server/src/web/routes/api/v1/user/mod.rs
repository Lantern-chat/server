use ftl::*;
use sdk::Snowflake;

use crate::{web::auth::authorize, Error};

use super::ApiResponse;

//pub mod check;
pub mod profile;
pub mod register;

pub mod me;

pub async fn user(mut route: Route<crate::ServerState>) -> ApiResponse {
    match route.next().method_segment() {
        // POST /api/v1/user
        (&Method::POST, End) => register::register(route).await,

        // ANY /api/v1/user/@me
        (_, Exact("@me")) => me::me(route).await,

        // ANY /api/v1/user/1234
        (_, Exact(segment)) => {
            let Ok(user_id) = segment.parse::<Snowflake>() else {
                return Err(Error::BadRequest);
            };

            let auth = authorize(&route).await?;

            match route.next().method_segment() {
                (&Method::GET, Exact("profile")) => profile::profile(route, auth, user_id).await,
                _ => Err(Error::Unimplemented),
            }
        }
        _ => Err(Error::NotFound),
    }
}
