use std::{convert::Infallible, net::SocketAddr};

use hyper::{Body, Request, Response};

use super::{ftl::Route, routes::routes, ServerState};

pub async fn service(
    addr: SocketAddr,
    req: Request<Body>,
    state: ServerState,
) -> Result<Response<Body>, Infallible> {
    // skip leading slashes
    let segment_index = req.uri().path().starts_with('/') as usize;

    let resp = routes(Route {
        addr,
        req,
        state,
        segment_index,
        has_body: true,
    })
    .await;

    Ok(resp)
}
