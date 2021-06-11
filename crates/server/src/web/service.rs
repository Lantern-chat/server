use std::{convert::Infallible, net::SocketAddr};

use headers::HeaderValue;
use hyper::{Body, Request, Response};

use crate::{web::routes, ServerState};
use ftl::*;

pub async fn service(
    addr: SocketAddr,
    req: Request<Body>,
    state: ServerState,
) -> Result<Response<Body>, Infallible> {
    //if state.ip_bans.is_probably_banned(addr.ip()) {
    //    let check = async {};
    //
    //    match check.await {
    //        Ok(false) => {}
    //        Ok(true) => return Ok(StatusCode::FORBIDDEN.into_response()),
    //        Err(e) => {
    //            log::error!("Error checking if IP is banned")
    //        }
    //    };
    //}

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

    log::debug!("{} -> {} {:.4}ms", info, resp.status(), elapsed);
    if let Ok(value) = HeaderValue::from_str(&format!("resp;dur={:.4}", elapsed)) {
        resp.headers_mut().insert("Server-Timing", value);
    }

    Ok(resp)
}
