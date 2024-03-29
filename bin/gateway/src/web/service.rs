use std::{
    convert::{Infallible, TryFrom},
    net::SocketAddr,
};

use headers::{HeaderName, HeaderValue};
use hyper::{body::Incoming, Request, Response};

use ftl::*;

use crate::prelude::*;

//use crate::{metrics::API_METRICS, web::routes, ServerState};

pub async fn service(
    addr: SocketAddr,
    req: Request<Incoming>,
    state: ServerState,
) -> Result<Response<Body>, Infallible> {
    let route = Route::new(addr, req, state);

    let mut info = String::new();
    if log::level_enabled!(log::Level::DEBUG) {
        info = format!(
            "{:?}: {} http://{}{}",
            route.real_addr.ip(),
            route.method(),
            match route.host() {
                Some(ref h) => h.as_str(),
                None => "unknown",
            },
            route.uri()
        );
    }

    let start = route.start;

    let mut resp = crate::web::routes::entry(route).await;

    let elapsed = start.elapsed();
    let elapsedf = elapsed.as_secs_f64() * 1_000.0;
    let status = resp.status();

    let headers = resp.headers_mut();

    // http://www.gnuterrypratchett.com/
    headers.insert(
        HeaderName::from_static("x-clacks-overhead"),
        HeaderValue::from_static("GNU Terry Pratchett"),
    );

    log::debug!("{info} -> {} {:.4}ms", status, elapsedf);
    if let Ok(value) = HeaderValue::try_from({
        use std::fmt::Write;

        // reuse the info string to avoid another allocation
        info.clear();

        if let Err(e) = write!(info, "resp;dur={elapsedf:.4}") {
            log::error!("Error formatting response duration: {e}");
        }

        info
    }) {
        headers.insert(HeaderName::from_static("server-timing"), value);
    }

    //API_METRICS.load().add_req(resp.status().as_u16(), elapsed);

    Ok(resp)
}
