use std::{net::SocketAddr, sync::Arc};

use warp::{hyper::Server, reject::Reject, Filter, Rejection, Reply};

use crate::{
    db::Snowflake,
    server::{rate::RateLimitKey, routes::filters::real_ip, ServerState},
};

mod build;
mod file;
mod party;
mod user;

pub fn status() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("status").map(|| "Testing")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum Route {
    Status,
    Build,
    User,
    Party,
}

#[derive(Debug)]
pub struct RateLimited;
impl Reject for RateLimited {}

pub fn rate_limit(
    route: Route,
    state: Arc<ServerState>,
) -> impl Filter<Extract = (), Error = Rejection> + Clone {
    warp::any()
        .map(move || state.clone())
        .and(real_ip())
        .and_then(move |state: Arc<ServerState>, ip: SocketAddr| async move {
            let allowed = state
                .rate_limit
                .req(RateLimitKey {
                    ip,
                    account: Snowflake::null(), // TODO: Get account from cookies?
                    route: route as u16,
                })
                .await;

            if allowed {
                Ok(())
            } else {
                Err(warp::reject::custom(RateLimited))
            }
        })
        .untuple_one()
}

pub fn api(
    state: Arc<ServerState>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    balanced_or_tree!(
        rate_limit(Route::Status, state.clone()).and(status()),
        rate_limit(Route::Build, state.clone()).and(build::route()),
        rate_limit(Route::User, state.clone()).and(user::user(state.clone())),
        rate_limit(Route::Party, state.clone()).and(party::party(state.clone()))
    )
}
