use std::borrow::Cow;
use std::sync::Arc;

use warp::{
    body::json,
    hyper::{Server, StatusCode},
    reject::Reject,
    Filter, Rejection, Reply,
};

use crate::{
    db::{Client, ClientError, Snowflake},
    server::{auth::AuthToken, rate::RateLimitKey, routes::error::ApiError, ServerState},
};

pub fn check(
    state: ServerState,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("check")
        .and(crate::server::routes::filters::auth(state))
        .map(|_| warp::reply::reply())
        .recover(ApiError::recover)
}
