use std::any::{Any, TypeId};

use ftl::*;
use futures::future::BoxFuture;
use futures::Future;
use headers::ContentType;
use http::StatusCode;
use sdk::driver::Encoding;

use crate::web::encoding::EncodingQuery;
use crate::{Error, ServerState};

use ftl::reply::deferred::*;

pub enum WebResponse {
    Status(StatusCode),
    Single(StatusCode, DeferredValue),
    Stream(StatusCode, DeferredStream),
    Raw(Response),
}

impl WebResponse {
    #[inline]
    pub fn new<T>(value: T) -> WebResponse
    where
        T: serde::Serialize + Send + 'static,
    {
        WebResponse::Single(StatusCode::OK, DeferredValue::new(value))
    }

    #[inline]
    pub fn stream<T>(stream: impl futures::Stream<Item = Result<T, Error>> + Send + 'static) -> WebResponse
    where
        T: serde::Serialize + Send + Sync + 'static,
    {
        WebResponse::Stream(StatusCode::OK, DeferredStream::new(stream))
    }

    #[inline]
    pub fn with_status(self, status: StatusCode) -> Self {
        match self {
            WebResponse::Status(_) => WebResponse::Status(status),
            WebResponse::Single(_, v) => WebResponse::Single(status, v),
            WebResponse::Stream(_, v) => WebResponse::Stream(status, v),
            WebResponse::Raw(r) => WebResponse::Raw(r.with_status(status).into_response()),
        }
    }
}

impl<T> From<T> for WebResponse
where
    T: Reply + Any,
{
    fn from(value: T) -> Self {
        // poor-mans specialization
        match TypeId::of::<T>() {
            ty if ty == TypeId::of::<()>() => WebResponse::Status(StatusCode::OK),
            ty if ty == TypeId::of::<StatusCode>() => {
                WebResponse::Status(unsafe { std::mem::transmute_copy(&value) })
            }
            _ => WebResponse::Raw(value.into_response()),
        }
    }
}

pub type WebResult = Result<WebResponse, Error>;
pub type RouteResult = Result<BoxFuture<'static, WebResult>, Error>;

// we'll use this in multiple places, and the structs it accepts/returns are decently small,
// so avoid code duplication by marking it as never-inline
#[inline(never)]
pub fn web_response(encoding: Encoding, res: WebResult) -> Response {
    let res = res.and_then(|r| {
        Ok(match r {
            WebResponse::Status(s) => s.into_response(),
            WebResponse::Single(status, value) => {
                let (buf, ct) = match encoding {
                    Encoding::JSON => (value.as_json()?.into_bytes(), ContentType::json()),
                    Encoding::CBOR => (value.as_cbor()?, ftl::APPLICATION_CBOR.clone()),
                };

                hyper::Body::from(buf)
                    .with_header(ct)
                    .with_status(status)
                    .into_response()
            }
            WebResponse::Stream(status, stream) => match encoding {
                Encoding::JSON => stream.as_json().with_status(status).into_response(),
                Encoding::CBOR => stream.as_cbor().with_status(status).into_response(),
            },
            WebResponse::Raw(raw) => raw,
        })
    });

    match res {
        Ok(res) => res,
        Err(e) => e.into_encoding(encoding),
    }
}

pub async fn wrap_response<F, S, R>(route: Route<S>, cb: F) -> Response
where
    F: FnOnce(Route<S>) -> R,
    R: Future<Output = WebResult>,
{
    let encoding = match route.query::<crate::web::encoding::EncodingQuery>() {
        Some(Ok(q)) => q.encoding,
        _ => sdk::driver::Encoding::JSON,
    };

    web_response(encoding, cb(route).await)
}

pub fn wrap_response_once<S>(route: &Route<S>, cb: impl FnOnce(&Route<S>) -> WebResult) -> Response {
    let encoding = match route.query::<crate::web::encoding::EncodingQuery>() {
        Some(Ok(q)) => q.encoding,
        _ => sdk::driver::Encoding::JSON,
    };

    web_response(encoding, cb(route))
}
