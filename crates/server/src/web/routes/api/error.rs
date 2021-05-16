use std::borrow::Cow;

use ftl::{
    reply::{self, Reply},
    StatusCode,
};

use crate::ctrl::error::Error;

#[derive(Serialize)]
pub struct ApiError {
    pub code: u16,
    pub message: Cow<'static, str>,
}

impl ApiError {
    pub fn err(kind: Error) -> impl Reply {
        reply::json(&ApiError {
            message: kind.format(),
            code: kind.code(),
        })
        .with_status(kind.http_status())
    }
}
