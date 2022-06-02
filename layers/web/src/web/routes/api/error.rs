use std::borrow::Cow;

use ftl::{
    reply::{self, Reply},
    ReplyError, StatusCode,
};

use crate::ctrl::error::Error;

#[derive(Serialize)]
pub struct ApiError {
    pub code: u16,
    pub message: Cow<'static, str>,
}

use reply::{Json, WithStatus};

impl Reply for Error {
    fn into_response(self) -> ftl::Response {
        ApiError::err(self).into_response()
    }
}

impl ReplyError for Error {
    fn status(&self) -> StatusCode {
        self.http_status()
    }

    fn into_error_response(self) -> ftl::Response {
        ApiError::err(self).into_response()
    }
}

impl ApiError {
    fn real_err(kind: Error) -> WithStatus<Json> {
        if kind.is_fatal() {
            log::error!("Fatal error: {kind}");
        } else if cfg!(debug_assertions) {
            log::warn!("Non-fatal error: {kind}");
        }

        reply::json(&ApiError {
            message: kind.format(),
            code: kind.code(),
        })
        .with_status(kind.http_status())
    }

    pub fn err(kind: Error) -> WithStatus<Json> {
        lazy_static::lazy_static! {
            static ref NOT_FOUND: WithStatus<Json> = ApiError::real_err(Error::NotFound);
            static ref BAD_REQUEST: WithStatus<Json> = ApiError::real_err(Error::BadRequest);
            static ref UNAUTHORIZED: WithStatus<Json> = ApiError::real_err(Error::NoSession);
        }

        // use cached responses where possible
        match kind {
            Error::NoSession => UNAUTHORIZED.clone(),
            Error::NotFound => NOT_FOUND.clone(),
            Error::BadRequest => BAD_REQUEST.clone(),
            _ => Self::real_err(kind),
        }
    }

    pub fn unauthorized() -> WithStatus<Json> {
        Self::err(Error::NoSession)
    }

    pub fn not_found() -> WithStatus<Json> {
        Self::err(Error::NotFound)
    }

    pub fn bad_request() -> WithStatus<Json> {
        Self::err(Error::BadRequest)
    }
}
