#[macro_use]
mod macros;

use std::sync::Arc;

use warp::{Filter, Rejection, Reply};

use crate::state::ServerState;

pub mod api;
pub mod gateway;

pub fn routes(
    state: Arc<ServerState>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    balanced_or_tree!(api::api(), gateway::gateway(state.clone())).with(warp::log("server"))
}
