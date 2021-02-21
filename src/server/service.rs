use std::{convert::Infallible, net::SocketAddr, time::Instant};

use hyper::{Body, Request, Response};

use super::{ftl::*, routes::routes, ServerState};

pub async fn service(
    addr: SocketAddr,
    req: Request<Body>,
    state: ServerState,
) -> Result<Response<Body>, Infallible> {
    // skip leading slashes
    let segment_index = req.uri().path().starts_with('/') as usize;

    let mut route = Route {
        addr,
        req,
        state,
        segment_index,
        has_body: true,
    };

    let info = format!(
        "{:?}: {} {}",
        real_ip::get_real_ip(&route),
        route.req.method(),
        route.req.uri()
    );

    let now = Instant::now();

    let resp = routes(route).await;

    let elapsed = now.elapsed().as_secs_f64() * 1000.0;

    log::info!("{} -> {} {:.4}ms", info, resp.status(), elapsed);

    Ok(resp)
}
