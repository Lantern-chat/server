use std::sync::Arc;
use std::{borrow::Cow, convert::Infallible};

use warp::{
    body::BodyDeserializeError,
    hyper::StatusCode,
    reply::{Json, WithStatus},
    Filter, Rejection, Reply,
};

use crate::server::ServerState;

#[derive(Serialize)]
pub struct ApiError {
    pub code: u16,
    pub message: Cow<'static, str>,
}

impl ApiError {
    pub fn err(status: StatusCode, message: Cow<'static, str>) -> WithStatus<Json> {
        warp::reply::with_status(
            warp::reply::json(&ApiError {
                message,
                code: status.as_u16(),
            }),
            status,
        )
    }

    pub fn ok<T: serde::Serialize>(value: &T) -> WithStatus<Json> {
        warp::reply::with_status(warp::reply::json(value), StatusCode::OK)
    }

    pub async fn recover(err: Rejection) -> Result<impl Reply, Rejection> {
        let code;
        let message;

        if err.is_not_found() {
            return Err(err);
        } else if err.find::<super::wrappers::RateLimited>().is_some() {
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