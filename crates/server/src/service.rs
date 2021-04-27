use std::{convert::Infallible, net::SocketAddr};

use headers::HeaderValue;
use hyper::{Body, Request, Response};

use super::{routes, ServerState};
use ftl::*;

pub async fn service(
    addr: SocketAddr,
    req: Request<Body>,
    state: ServerState,
) -> Result<Response<Body>, Infallible> {
    let route = Route::new(addr, req, state);

    let info = format!(
        "{:?}: {} {}",
        real_ip::get_real_ip(&route),
        route.req.method(),
        route.req.uri()
    );

    let start = route.start;

    let mut resp = compression::wrap_route(false, route, |r| routes::entry(r)).await;

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;

    log::info!("{} -> {} {:.4}ms", info, resp.status(), elapsed);
    if let Ok(value) = HeaderValue::from_str(&format!("resp;dur={:.4}", elapsed)) {
        resp.headers_mut().insert("Server-Timing", value);
    }

    Ok(resp)
}
