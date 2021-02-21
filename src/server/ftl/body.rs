use bytes::Buf;
use headers::{ContentLength, ContentType, HeaderMapExt};
use http::StatusCode;

use super::{BodyError, Route};

#[derive(Debug, thiserror::Error)]
pub enum BodyDeserializeError {
    #[error("{0}")]
    BodyError(#[from] BodyError),

    #[error("Parse Error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Parse Error: {0}")]
    Form(#[from] serde_urlencoded::de::Error),

    #[error("Content Type Error")]
    IncorrectContentType,
}

pub async fn json<T>(route: &mut Route) -> Result<T, BodyDeserializeError>
where
    T: serde::de::DeserializeOwned,
{
    if route.header::<ContentType>() != Some(ContentType::json()) {
        return Err(BodyDeserializeError::IncorrectContentType);
    }

    let body = route.aggregate().await?;

    Ok(serde_json::from_reader(body.reader())?)
}

pub async fn form<T>(route: &mut Route) -> Result<T, BodyDeserializeError>
where
    T: serde::de::DeserializeOwned,
{
    match route.header::<ContentType>() {
        Some(ct) if ct == ContentType::form_url_encoded() => {}
        _ => return Err(BodyDeserializeError::IncorrectContentType),
    }

    let body = route.aggregate().await?;

    Ok(serde_urlencoded::from_reader(body.reader())?)
}

use http::Response;
use hyper::Body;

impl Reply for BodyDeserializeError {
    fn into_response(self) -> Response<Body> {
        match self {
            BodyDeserializeError::IncorrectContentType => "Incorrect Content-Type"
                .with_status(StatusCode::BAD_REQUEST)
                .into_response(),
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub fn content_length(route: &Route) -> Option<u64> {
    route.header::<ContentLength>().map(|cl| cl.0)
}

use super::reply::Reply;

pub fn content_length_limit(route: &Route, limit: u64) -> Option<impl Reply> {
    match content_length(route) {
        Some(len) if len > limit => Some("Content length is too long"),
        None => Some("Content-length is missing!"),
        _ => None,
    }
}
