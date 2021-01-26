use warp::{Filter, Rejection, Reply};

/// Index will display 404 info client-side on load
pub fn route() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let index = warp::fs::file("frontend/dist/index.html");
    let files = warp::path("static").and(warp::fs::dir("frontend/dist"));

    warp::get().and(files.or(index))
}
