use std::{net::SocketAddr, sync::Arc};

use log::callsite::register;
use warp::{hyper::Server, reject::Reject, Filter, Rejection, Reply};

use crate::{
    db::Snowflake,
    server::{rate::RateLimitKey, routes::filters::real_ip, ServerState},
};

mod user {
    pub mod check;
    pub mod login;
    pub mod logout;
    pub mod register;
}

pub fn api(state: ServerState) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let user_routes = warp::path("user").and({
        let user_post_routes = warp::post().and(balanced_or_tree!(
            // POST /api/v1/user/login
            warp::path("login").and(user::login::login(state.clone())),
            // POST /api/v1/user
            warp::path::end().and(user::register::register(state.clone()))
        ));

        let user_delete_routes = warp::delete().and(balanced_or_tree!(
            // DELETE /api/v1/user/logout
            warp::path("logout").and(user::logout::logout(state.clone())),
        ));

        balanced_or_tree!(user_post_routes, user_delete_routes)
    });

    let party_routes = warp::path("party").and(balanced_or_tree!(
        warp::any() //gsdg
    ));

    balanced_or_tree!(user_routes)
}

#[derive(Debug)]
pub struct RateLimited;
impl Reject for RateLimited {}
