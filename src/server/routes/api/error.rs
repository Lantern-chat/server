use std::borrow::Cow;

use http::StatusCode;

use crate::server::reply::{json, Reply};

#[derive(Serialize)]
pub struct ApiError {
    pub code: u16,
    pub message: Cow<'static, str>,
}

impl ApiError {
    pub fn err(status: StatusCode, message: impl Into<Cow<'static, str>>) -> impl Reply {
        json(&ApiError {
            message: message.into(),
            code: status.as_u16(),
        })
        .with_status(status)
    }
}
