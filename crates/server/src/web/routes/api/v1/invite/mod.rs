pub mod get;
pub mod redeem;
pub mod revoke;

use ftl::*;
use smol_str::SmolStr;

use crate::{
    ctrl::Error,
    web::{auth::authorize, routes::api::ApiError},
    ServerState,
};

pub async fn invite(mut route: Route<ServerState>) -> Response {
    let auth = match authorize(&route).await {
        Ok(auth) => auth,
        Err(e) => return ApiError::err(e).into_response(),
    };

    if let Exact(_) = route.next().segment() {
        match route.param::<SmolStr>() {
            Some(Ok(code)) => match route.next().method_segment() {
                (&Method::GET, End) => get::get_invite(route, auth, code).await,
                (&Method::POST, Exact("redeem")) => redeem::redeem(route, auth, code).await,
                (&Method::DELETE, Exact("revoke")) => revoke::revoke(route, auth, code).await,

                _ => ApiError::not_found().into_response(),
            },
            Some(Err(_)) => ApiError::bad_request().into_response(),
            None => ApiError::not_found().into_response(),
        }
    } else {
        ApiError::not_found().into_response()
    }
}
