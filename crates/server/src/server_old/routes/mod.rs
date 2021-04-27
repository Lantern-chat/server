#[macro_use]
mod macros;

use std::sync::Arc;

use warp::{Filter, Rejection, Reply};

use crate::ServerState;

pub mod wrappers;

pub mod api;
pub mod assets;
pub mod error;
pub mod filters;
pub mod gateway;

pub fn routes(
    state: ServerState,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let routes = balanced_or_tree!(
        api::api(state.clone()),
        gateway::gateway(state.clone()),
        assets::files::route(), // ensure this is last, as it has a wildcard to return index
    )
    .with(warp::cors().build());

    //#[cfg(debug_assertions)]
    return routes.with(warp::trace::request());

    //#[cfg(not(debug_assertions))]
    //routes
}
