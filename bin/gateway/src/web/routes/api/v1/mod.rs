pub mod admin;
pub mod gateway;
pub mod invite;
pub mod oembed;
pub mod party;
pub mod room;
pub mod user;

#[cfg(debug_assertions)]
pub mod debug;

use super::*;

// import all these to be used by child modules
use crate::prelude::*;
use crate::web::auth::MaybeAuth;
use sdk::driver::Encoding;

use futures::future::BoxFuture;
use rpc::{client::RpcClientError, procedure::Procedure, request::RpcRequest};

pub type ApiResult = Result<Procedure, Error>;
pub type RouteResult = Result<BoxFuture<'static, ApiResult>, Error>;

async fn api_v1_inner(
    mut route: Route<ServerState>,
    state: &ServerState,
    encoding: Encoding,
) -> Result<Response, Error> {
    let addr = route.real_addr;

    // only `PATCH api/v1/file` is allowed to exceed this value
    // if *route.method() != Method::PATCH || route.segment() != Exact("file") {}

    route.next();

    if Exact("gateway") == route.segment() {
        return gateway::gateway(route);
    }

    if let Some(body) = route.body() {
        use hyper::body::Body;

        // API Requests are limited to a body size of 1MiB
        // TODO: Reduce this eventually?
        const MAX_BODY_SIZE: u64 = 1024 * 1024;

        let size_hint = body.size_hint();

        if size_hint.lower() >= MAX_BODY_SIZE {
            return Err(Error::RequestEntityTooLarge);
        }

        if matches!(size_hint.upper(), Some(len) if len >= MAX_BODY_SIZE) {
            return Err(Error::RequestEntityTooLarge);
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

        _ => return Err(Error::NotFoundSignaling),
    };

    let cmd = RpcRequest::Procedure {
        proc: route_res?.await?,
        auth: auth.0,
        addr,
    };

    match state.rpc.send(&cmd).await {
        // penalize for non-existent resources
        Err(RpcClientError::DoesNotExist) => Err(Error::NotFoundHighPenalty),
        Err(e) => {
            log::error!("Error sending RPC request: {:?}", e);
            Err(Error::InternalErrorStatic("RPC Error"))
        }
        Ok(recv) => {
            let RpcRequest::Procedure { ref proc, .. } = cmd else {
                unreachable!()
            };

            proc.stream_response(recv, encoding).await
        }
    }
}

pub async fn api_v1(route: Route<ServerState>) -> Response {
    let state = route.state.clone();
    let addr = route.real_addr;

    let encoding = match route.query::<crate::web::encoding::EncodingQuery>() {
        Some(Ok(q)) => q.encoding,
        _ => sdk::driver::Encoding::JSON,
    };

    match api_v1_inner(route, &state, encoding).await {
        Ok(resp) => resp,
        Err(e) => {
            let penalty = e.penalty();

            if penalty > 0 {
                state.rate_limit.penalize(&state, addr, penalty).await;
            }

            e.into_encoding(encoding)
        }
    }
}
