pub mod admin;
pub mod invite;
pub mod oembed;
pub mod party;
pub mod room;
pub mod user;

#[cfg(debug_assertions)]
pub mod debug;

use super::*;

use futures::{future::BoxFuture, Future};
use rpc::msg::Procedure;

pub struct RawMessage {
    pub proc: Procedure,
    pub auth: Option<Authorization>,
}

impl RawMessage {
    #[inline]
    pub fn authorized(auth: Authorization, proc: impl Into<Procedure>) -> Self {
        RawMessage {
            proc: proc.into(),
            auth: Some(auth),
        }
    }

    #[inline]
    pub fn unauthorized(proc: impl Into<Procedure>) -> Self {
        RawMessage {
            proc: proc.into(),
            auth: None,
        }
    }
}

pub type ApiResult = Result<RawMessage, crate::error::Error>;
pub type RouteResult = Result<BoxFuture<'static, ApiResult>, crate::error::Error>;

async fn api_v1_inner(mut route: Route<ServerState>) -> ApiResult {
    route.next();

    // only `PATCH api/v1/file` is allowed to exceed this value
    if !(*route.method() == Method::PATCH && route.segment() == Exact("file")) {
        use hyper::body::Body;

        if let Some(body) = route.body() {
            // API Requests are limited to a body size of 1MiB
            // TODO: Reduce this eventually?
            if matches!(body.size_hint().upper(), Some(len) if len >= (1024 * 1024)) {
                return Err(Error::RequestEntityTooLarge);
            }
        }
    }

    let auth = crate::web::auth::maybe_authorize(&route).await?;

    let route_res = match route.method_segment() {
        (_, Exact("room")) => room::room(route, auth),
        (_, Exact("user")) => user::user(route, auth),
        (_, Exact("party")) => party::party(route, auth),
        (_, Exact("invite")) => invite::invite(route, auth),
        (&Method::GET, Exact("oembed")) => Ok(oembed::oembed(route)),
        (_, Exact("admin")) => admin::admin(route, auth),

        #[cfg(debug_assertions)]
        (_, Exact("debug")) => debug::debug(route),

        _ => return Err(Error::NotFound),
    };

    route_res?.await
}

pub async fn api_v1(route: Route<ServerState>) -> Response {
    let addr = route.real_addr;
    let state = route.state.clone();

    let raw = api_v1_inner(route).await;

    unimplemented!()
}
