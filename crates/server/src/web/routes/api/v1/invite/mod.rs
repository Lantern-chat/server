pub mod get;
pub mod post;
pub mod redeem;
pub mod revoke;

use ftl::*;

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

    match route.next().method_segment() {
        (&Method::POST, End) => post::post(route, auth).await,
        (_, Exact(_)) => match route.param::<String>() {
            Some(Ok(code)) => match route.next().method_segment() {
                (&Method::GET, End) => get::get_invite(route, auth, code).await,
                (&Method::POST, Exact("redeem")) => redeem::redeem(route, auth, code).await,
                (&Method::DELETE, Exact("revoke")) => revoke::revoke(route, auth, code).await,

                _ => ApiError::not_found().into_response(),
            },
            Some(Err(_)) => return ApiError::bad_request().into_response(),
            None => return ApiError::not_found().into_response(),
        },
        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
    }
}
