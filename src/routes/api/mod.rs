use warp::{Filter, Rejection, Reply};

pub fn status() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("status").map(|| "Testing")
}

pub fn api() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("api").and(balanced_or_tree!(status()))
}
