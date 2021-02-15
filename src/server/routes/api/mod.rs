use std::sync::Arc;
use std::{borrow::Cow, convert::Infallible};

use warp::{body::BodyDeserializeError, hyper::StatusCode, Filter, Rejection, Reply};

pub mod v1;

use crate::server::ServerState;

#[derive(Serialize)]
pub struct ApiError {
    code: u16,
    message: Cow<'static, str>,
}

impl ApiError {
    pub fn reply_json(status: StatusCode, value: &'static str) -> impl Reply {
        warp::reply::with_status(
            warp::reply::json(&ApiError {
                message: value.into(),
                code: status.as_u16(),
            }),
            status,
        )
    }

    pub async fn recover(err: Rejection) -> Result<impl Reply, Rejection> {
        let code;
        let message;

        if err.is_not_found() {
            return Err(err);
        } else if err.find::<v1::RateLimited>().is_some() {
            code = StatusCode::TOO_MANY_REQUESTS;
            message = "TOO_MANY_REQUESTS";
        } else if err.find::<super::filters::NoAuth>().is_some() {
            code = StatusCode::UNAUTHORIZED;
            message = "UNAUTHORIZED";
        } else if err.find::<BodyDeserializeError>().is_some() {
            code = StatusCode::BAD_REQUEST;
            message = "BAD_REQUEST";
        } else {
            code = StatusCode::INTERNAL_SERVER_ERROR;
            message = "INTERNAL_SERVER_ERROR";
        }

        Ok(warp::reply::with_status(
            warp::reply::json(&ApiError {
                code: code.as_u16(),
                message: message.into(),
            }),
            code,
        ))
    }
}

pub fn api(state: ServerState) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let apis = warp::path("v1").and(v1::api(state));

    warp::path("api").and(apis).recover(ApiError::recover)
}
