#[macro_use]
mod macros;

use std::sync::Arc;

use warp::{Filter, Rejection, Reply};

use crate::server::ServerState;

pub mod api;
pub mod gateway;

pub fn routes(
    state: Arc<ServerState>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    balanced_or_tree!(api::api(state.clone()), gateway::gateway(state.clone()))
        .with(warp::log("server"))
}
