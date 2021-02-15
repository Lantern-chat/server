use std::sync::Arc;

use log::callsite::register;
use warp::{hyper::Server, reject::Reject, Filter, Rejection, Reply};

use crate::{
    db::Snowflake,
    server::{rate::RateLimitKey, ServerState},
};

pub fn rooms(
    state: ServerState,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path::param::<Snowflake>().and(warp::path("rooms"));

    warp::path!(Snowflake / "rooms").map(|party_id| warp::reply::reply())
}
