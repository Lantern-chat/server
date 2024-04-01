pub mod admin;
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
use sdk::Snowflake;

use futures::{future::BoxFuture, Future};
use rpc::{procedure::Procedure, request::RpcRequest};

pub type ApiResult = Result<Procedure, Error>;
pub type RouteResult = Result<BoxFuture<'static, ApiResult>, Error>;

async fn api_v1_inner(mut route: Route<ServerState>) -> Result<RpcRequest, Error> {
    route.next();

    let addr = route.real_addr;

    // only `PATCH api/v1/file` is allowed to exceed this value
    // if *route.method() != Method::PATCH || route.segment() != Exact("file") {}

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

    Ok(RpcRequest::Procedure {
        proc: route_res?.await?,
        auth: auth.0,
        addr,
    })
}

pub async fn api_v1(route: Route<ServerState>) -> Response {
    use rpc::client::RpcClientError;

    let state = route.state.clone();
    let addr = route.real_addr;

    let encoding = match route.query::<crate::web::encoding::EncodingQuery>() {
        Some(Ok(q)) => q.encoding,
        _ => sdk::driver::Encoding::JSON,
    };

    let res = match api_v1_inner(route).await {
        Ok(cmd) => match state.rpc.send(&cmd).await {
            // penalize for non-existent resources
            Err(RpcClientError::DoesNotExist) => Err((500, Error::NotFound)),
            Err(e) => {
                log::error!("Error sending RPC request: {:?}", e);
                Err((0, Error::InternalErrorStatic("RPC Error")))
            }
            Ok(recv) => {
                let RpcRequest::Procedure { ref proc, .. } = cmd else {
                    unreachable!()
                };

                match proc.stream_response(recv, encoding).await {
                    Ok(resp) => Ok(resp),
                    Err(e) => Err((0, e)),
                }
            }
        },
        Err(e) => Err((
            // Rate-limiting penalty in milliseconds
            // TODO: Make this configurable or find better values
            match e {
                Error::NotFoundSignaling => 100,
                Error::BadRequest => 200,
                Error::Unauthorized => 200,
                Error::MethodNotAllowed => 200,
                _ => 0,
            },
            e,
        )),
    };

    match res {
        Ok(resp) => resp,
        Err((penalty, e)) => {
            if penalty > 0 {
                state.rate_limit.penalize(&state, addr, penalty).await;
            }

            e.into_encoding(encoding)
        }
    }
}
