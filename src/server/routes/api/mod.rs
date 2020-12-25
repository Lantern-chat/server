use std::sync::Arc;

use warp::{hyper::StatusCode, Filter, Rejection, Reply};

pub mod v1;

use crate::server::ServerState;

#[derive(Serialize)]
pub struct ApiError {
    code: u16,
    message: String,
}

pub fn api(
    state: Arc<ServerState>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let apis = warp::path("v1").and(v1::api(state));

    warp::path("api")
        .and(apis)
        .recover(|err: Rejection| async move {
            let code;
            let message;

            if err.is_not_found() {
                return Err(err);
            } else if err.find::<v1::RateLimited>().is_some() {
                code = StatusCode::TOO_MANY_REQUESTS;
                message = "RATE_LIMITED";
            } else {
                code = StatusCode::INTERNAL_SERVER_ERROR;
                message = "UNHANDLED_REJECTION";
            }

            let json = warp::reply::json(&ApiError {
                code: code.as_u16(),
                message: message.to_owned(),
            });

            Ok(warp::reply::with_status(json, code))
        })
}
