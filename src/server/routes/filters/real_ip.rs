use std::{net::SocketAddr, sync::Arc};

use warp::{hyper::Server, reject::Reject, Filter, Rejection, Reply};

use crate::{
    db::{ClientError, Snowflake},
    server::{auth::AuthToken, rate::RateLimitKey, ServerState},
};

pub fn real_ip() -> impl Filter<Extract = (SocketAddr,), Error = Rejection> + Clone {
    warp::header("x-real-ip")
        .or(warp::header("x-forwarded-for"))
        .unify()
        .or(warp::addr::remote().and_then(|addr| async move {
            match addr {
                Some(addr) => Ok(addr),
                None => Err(warp::reject()),
            }
        }))
        .unify()
}
