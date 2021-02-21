use std::borrow::Cow;

use http::StatusCode;

use crate::server::ftl::reply::{self, Reply};

#[derive(Serialize)]
pub struct ApiError {
    pub code: u16,
    pub message: Cow<'static, str>,
}

impl ApiError {
    pub fn err(status: StatusCode, message: impl Into<Cow<'static, str>>) -> impl Reply {
        reply::json(&ApiError {
            message: message.into(),
            code: status.as_u16(),
        })
        .with_status(status)
    }
}
