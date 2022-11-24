use std::{convert::Infallible, net::SocketAddr};

use headers::HeaderValue;
use hyper::{Body, Request, Response};

use crate::{metrics::API_METRICS, web::routes, ServerState};
use ftl::*;

pub async fn service(
    addr: SocketAddr,
    req: Request<Body>,
    state: ServerState,
) -> Result<Response<Body>, Infallible> {
    let route = Route::new(addr, req, state);

    let mut info = String::new();
    if log::level_enabled!(log::Level::DEBUG) {
        info = format!(
            "{:?}: {} http://{}{}",
            route.real_addr.ip(),
            route.req.method(),
            match route.host() {
                Some(ref h) => h.as_str(),
                None => "unknown",
            },
            route.req.uri()
        );
    }

    let start = route.start;

    let mut resp = routes::entry(route).await;

    let elapsed = start.elapsed();
    let elapsedf = elapsed.as_secs_f64() * 1_000.0;

    log::debug!("{info} -> {} {:.4}ms", resp.status(), elapsedf);
    if let Ok(value) = HeaderValue::from_str({
        use std::fmt::Write;

        // reuse the info string to avoid another allocation
        info.clear();

        if let Err(e) = write!(info, "resp;dur={:.4}", elapsedf) {
            log::error!("Error formatting response duration: {e}");
        }

        &info
    }) {
        resp.headers_mut().insert("Server-Timing", value);
    }

    API_METRICS.load().add_req(resp.status().as_u16(), elapsed);

    Ok(resp)
}
