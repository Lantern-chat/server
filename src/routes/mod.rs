#[macro_use]
mod macros;

use std::sync::Arc;

use warp::{Filter, Rejection, Reply};

use crate::state::ServerState;

pub mod gateway;

pub fn index() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("/").map(|| "Tessting")
}

pub fn gateway(
    state: Arc<ServerState>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("gateway")
        .and(warp::ws())
        .and(warp::addr::remote())
        .and(warp::any().map(move || state.clone()))
        .map(|ws: warp::ws::Ws, addr, state| {
            ws.on_upgrade(move |socket| gateway::client_connected(socket, addr, state))
        })
}
