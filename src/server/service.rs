use std::{convert::Infallible, net::SocketAddr, time::Instant};

use hyper::{Body, Request, Response};

use super::{ftl::*, routes, ServerState};

pub async fn service(
    addr: SocketAddr,
    req: Request<Body>,
    state: ServerState,
) -> Result<Response<Body>, Infallible> {
    let mut route = Route::new(addr, req, state);

    let info = format!(
        "{:?}: {} {}",
        real_ip::get_real_ip(&route),
        route.req.method(),
        route.req.uri()
    );

    let now = Instant::now();

    let resp = routes::entry(route).await;

    let elapsed = now.elapsed().as_secs_f64() * 1000.0;

    log::info!("{} -> {} {:.4}ms", info, resp.status(), elapsed);

    Ok(resp)
}
