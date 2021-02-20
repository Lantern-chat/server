use std::sync::Arc;

use log::callsite::register;
use warp::{hyper::Server, reject::Reject, Filter, Rejection, Reply};

use crate::{
    db::Snowflake,
    server::{rate::RateLimitKey, ServerState},
};

mod create;

pub fn room(state: ServerState) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("room").and(balanced_or_tree!(create::create(state.clone())))
}
