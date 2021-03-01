use bytes::Buf;
use headers::{ContentLength, ContentType, HeaderMapExt};
use http::StatusCode;
use serde::de::DeserializeOwned;

use super::{BodyError, Reply, ReplyError, Response, Route};

pub async fn any<T>(route: &mut Route) -> Result<T, BodyDeserializeError>
where
    T: DeserializeOwned,
{
    let content_type = route.header::<ContentType>();

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum BodyType {
        Json,
        FormUrlEncoded,
        MsgPack,
    }

    let kind = if let Some(ct) = route.header::<ContentType>() {
        if ct == ContentType::json() {
            BodyType::Json
        } else if ct == ContentType::form_url_encoded() {
            BodyType::FormUrlEncoded
        } else if ct == ContentType::from(mime::APPLICATION_MSGPACK) {
            BodyType::MsgPack
        } else {
            return Err(BodyDeserializeError::IncorrectContentType);
        }
    } else {
        return Err(BodyDeserializeError::IncorrectContentType);
    };

    let reader = route.aggregate().await?.reader();

    Ok(match kind {
        BodyType::Json => serde_json::from_reader(reader)?,
        BodyType::FormUrlEncoded => serde_urlencoded::from_reader(reader)?,
        BodyType::MsgPack => rmp_serde::from_read(reader)?,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum BodyDeserializeError {
    #[error("{0}")]
    BodyError(#[from] BodyError),

    #[error("Parse Error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Parse Error: {0}")]
    Form(#[from] serde_urlencoded::de::Error),

    #[error("Parse Error: {0}")]
    MsgPack(#[from] rmp_serde::decode::Error),

    #[error("Content Type Error")]
    IncorrectContentType,
}

pub async fn json<T>(route: &mut Route) -> Result<T, BodyDeserializeError>
where
    T: DeserializeOwned,
{
    if route.header::<ContentType>() != Some(ContentType::json()) {
        return Err(BodyDeserializeError::IncorrectContentType);
    }

    let body = route.aggregate().await?;

    Ok(serde_json::from_reader(body.reader())?)
}

pub async fn form<T>(route: &mut Route) -> Result<T, BodyDeserializeError>
where
    T: DeserializeOwned,
{
    match route.header::<ContentType>() {
        Some(ct) if ct == ContentType::form_url_encoded() => {}
        _ => return Err(BodyDeserializeError::IncorrectContentType),
    }

    let body = route.aggregate().await?;

    Ok(serde_urlencoded::from_reader(body.reader())?)
}

pub async fn msgpack<T>(route: &mut Route) -> Result<T, BodyDeserializeError>
where
    T: DeserializeOwned,
{
    match route.header::<ContentType>() {
        Some(ct) if ct == ContentType::from(mime::APPLICATION_MSGPACK) => {}
        _ => return Err(BodyDeserializeError::IncorrectContentType),
    }

    let body = route.aggregate().await?;

    Ok(rmp_serde::from_read(body.reader())?)
}

impl Reply for BodyDeserializeError {
    fn into_response(self) -> Response {
        match self {
            BodyDeserializeError::IncorrectContentType => "Incorrect Content-Type"
                .with_status(StatusCode::BAD_REQUEST)
                .into_response(),
            _ => self.status().into_response(),
        }
    }
}

impl ReplyError for BodyDeserializeError {
    fn status(&self) -> StatusCode {
        match self {
            BodyDeserializeError::BodyError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        }
    }
}

pub fn content_length_limit(route: &Route, limit: u64) -> Option<impl Reply> {
    match route.content_length() {
        Some(len) if len > limit => Some("Content length is too long"),
        None => Some("Content-length is missing!"),
        _ => None,
    }
}
