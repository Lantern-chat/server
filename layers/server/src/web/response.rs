use std::any::{Any, TypeId};

use ftl::*;
use futures::future::BoxFuture;
use futures::Future;
use headers::ContentType;
use http::header::IntoHeaderName;
use http::{HeaderMap, HeaderValue, StatusCode};
use sdk::driver::Encoding;

use crate::Error;

use ftl::reply::deferred::*;

pub enum WebResponse {
    Status(StatusCode, Option<Box<HeaderMap>>),
    Single(StatusCode, Option<Box<HeaderMap>>, DeferredValue),
    Stream(StatusCode, Option<Box<HeaderMap>>, DeferredStream),
    Raw(Box<Response>),
}

impl WebResponse {
    #[inline]
    pub fn new<T>(value: T) -> WebResponse
    where
        T: serde::Serialize + Send + 'static,
    {
        WebResponse::Single(StatusCode::OK, None, DeferredValue::new(value))
    }

    #[inline]
    pub fn stream<T>(stream: impl futures::Stream<Item = Result<T, Error>> + Send + 'static) -> WebResponse
    where
        T: serde::Serialize + Send + Sync + 'static,
    {
        WebResponse::Stream(StatusCode::OK, None, DeferredStream::new(stream))
    }

    #[inline]
    pub fn with_status(self, status: StatusCode) -> Self {
        match self {
            WebResponse::Status(_, h) => WebResponse::Status(status, h),
            WebResponse::Single(_, h, v) => WebResponse::Single(status, h, v),
            WebResponse::Stream(_, h, v) => WebResponse::Stream(status, h, v),
            WebResponse::Raw(r) => WebResponse::Raw(Box::new(r.with_status(status).into_response())),
        }
    }

    #[inline]
    pub fn with_header(mut self, k: impl IntoHeaderName, v: HeaderValue) -> Self {
        match self {
            WebResponse::Status(_, ref mut headers)
            | WebResponse::Single(_, ref mut headers, _)
            | WebResponse::Stream(_, ref mut headers, _) => {
                headers.get_or_insert_with(|| Box::new(HeaderMap::new())).insert(k, v);
            }
            WebResponse::Raw(ref mut raw) => {
                raw.headers_mut().insert(k, v);
            }
        }

        self
    }
}

impl<T> From<T> for WebResponse
where
    T: Reply + Any,
{
    fn from(value: T) -> Self {
        // poor-mans specialization
        match TypeId::of::<T>() {
            ty if ty == TypeId::of::<()>() => WebResponse::Status(StatusCode::OK, None),
            ty if ty == TypeId::of::<StatusCode>() => {
                WebResponse::Status(unsafe { std::mem::transmute_copy(&value) }, None)
            }
            _ => WebResponse::Raw(Box::new(value.into_response())),
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
        let (mut resp, headers) = match r {
            WebResponse::Status(s, headers) => (s.into_response(), headers),
            WebResponse::Single(status, headers, value) => {
                let (buf, ct) = match encoding {
                    Encoding::JSON => (value.as_json()?.into_bytes(), ContentType::json()),
                    Encoding::CBOR => (value.as_cbor()?, ftl::APPLICATION_CBOR.clone()),
                };

                (
                    hyper::Body::from(buf).with_header(ct).with_status(status).into_response(),
                    headers,
                )
            }
            WebResponse::Stream(status, headers, stream) => (
                match encoding {
                    Encoding::JSON => stream.as_json().with_status(status).into_response(),
                    Encoding::CBOR => stream.as_cbor().with_status(status).into_response(),
                },
                headers,
            ),
            WebResponse::Raw(raw) => (*raw, None),
        };

        if let Some(headers) = headers {
            resp.headers_mut().extend(*headers);
        }

        Ok(resp)
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
