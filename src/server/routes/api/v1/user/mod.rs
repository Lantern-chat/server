use std::sync::Arc;

use log::callsite::register;
use warp::{hyper::Server, reject::Reject, Filter, Rejection, Reply};

use crate::{
    db::Snowflake,
    server::{rate::RateLimitKey, ServerState},
};

mod register;

pub fn user(
    state: Arc<ServerState>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    balanced_or_tree!(register::register(state.clone()))
}
