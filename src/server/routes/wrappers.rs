use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc,
    },
};

use warp::{hyper::Server, reject::Reject, Filter, Rejection, Reply};

use crate::{
    db::Snowflake,
    server::{rate::RateLimitKey, routes::filters::real_ip, ServerState},
};

use super::error::ApiError;

static ROUTE_COUNTER: AtomicU16 = AtomicU16::new(0);

#[derive(Debug)]
pub struct RateLimited;
impl Reject for RateLimited {}

pub fn rate_limit(
    state: &ServerState,
    req_per_sec: Option<u16>,
    route: impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let key = RateLimitKey {
        ip: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 8080)),
        req_per_sec: req_per_sec.unwrap_or(50),
        route_id: ROUTE_COUNTER.fetch_add(1, Ordering::SeqCst),
    };

    real_ip()
        .and(state.inject())
        .and_then(move |ip: SocketAddr, state: ServerState| async move {
            if state.rate_limit.req(RateLimitKey { ip, ..key }).await {
                Ok(())
            } else {
                Err(warp::reject::custom(RateLimited))
            }
        })
        .untuple_one()
        .and(route)
        .recover(ApiError::recover)
}
