use std::sync::Arc;

use log::callsite::register;
use warp::{hyper::Server, reject::Reject, Filter, Rejection, Reply};

use crate::{
    db::Snowflake,
    server::{rate::RateLimitKey, ServerState},
};

mod check;
mod login;
mod logout;
mod register;

pub fn user(state: ServerState) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("user").and(balanced_or_tree!(
        register::register(state.clone()),
        login::login(state.clone()),
        check::check(state.clone()),
        logout::logout(state.clone()),
    ))
}
