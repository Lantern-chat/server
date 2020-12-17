use std::sync::Arc;

use warp::{Filter, Rejection, Reply};

use crate::server::ServerState;

pub fn status() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("status").map(|| "Testing")
}

pub fn api(
    state: Arc<ServerState>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    balanced_or_tree!(status())
}
