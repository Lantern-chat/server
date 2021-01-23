use std::sync::Arc;

use warp::{
    body::json,
    hyper::{Server, StatusCode},
    reject::Reject,
    Filter, Rejection, Reply,
};

use crate::{
    db::{ClientError, Snowflake},
    server::{auth::AuthToken, rate::RateLimitKey, routes::api::ApiError, ServerState},
};

pub struct LoginForm {
    email_or_username: String,
    password: String,
}

pub fn login(
    state: Arc<ServerState>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("login").map(|| warp::reply::reply())
}
