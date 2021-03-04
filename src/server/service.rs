use std::{convert::Infallible, net::SocketAddr, time::Instant};

use futures::FutureExt;

use headers::HeaderValue;
use hyper::{body::HttpBody, Body, Request, Response};

use super::{ftl::*, routes, ServerState};

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

    let mut resp = routes::entry(route).await;

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;

    log::info!("{} -> {} {:.4}ms", info, resp.status(), elapsed);
    if let Ok(value) = HeaderValue::from_str(&format!("resp;dur={:.4}", elapsed)) {
        resp.headers_mut().insert("Server-Timing", value);
    }

    Ok(resp)
}
