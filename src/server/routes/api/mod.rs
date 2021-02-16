use std::sync::Arc;
use std::{borrow::Cow, convert::Infallible};

use warp::{body::BodyDeserializeError, hyper::StatusCode, Filter, Rejection, Reply};

pub mod util;
pub mod v1;

use crate::server::ServerState;

use super::error::ApiError;

pub fn api(state: ServerState) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let apis = warp::path("v1").and(v1::api(state));

    warp::path("api").and(apis).recover(ApiError::recover)
}
