use std::sync::Arc;

use warp::{Filter, Rejection, Reply};

pub mod v1;

use crate::server::ServerState;

pub fn api(
    state: Arc<ServerState>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("api").and(warp::path("v1").and(v1::api(state)))
}
