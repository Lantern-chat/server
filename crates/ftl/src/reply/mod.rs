use std::borrow::Borrow;

use headers::{ContentType, Header, HeaderMapExt};
use http::{Response as HttpResponse, StatusCode};
use hyper::Body;

pub mod json;
pub mod msgpack;

pub use json::{json, Json};

pub type Response = HttpResponse<Body>;

pub trait Reply: Sized {
    fn into_response(self) -> Response;

    #[inline]
    fn with_status(self, status: StatusCode) -> WithStatus<Self> {
        with_status(self, status)
    }

    #[inline]
    fn with_header<H>(self, header: H) -> WithHeader<Self, H>
    where
        H: Header,
    {
        WithHeader { reply: self, header }
    }
}

pub trait ReplyError: Reply {
    fn status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn into_error_response(self) -> Response {
        let status = self.status();
        self.with_status(status).into_response()
    }
}

impl<R, E> Reply for Result<R, E>
where
    R: Reply,
    E: ReplyError,
{
    fn into_response(self) -> Response {
        match self {
            Ok(reply) => reply.into_response(),
            Err(err) => err.into_error_response(),
        }
    }
}

impl<L, R> Reply for either::Either<L, R>
where
    L: Reply,
    R: Reply,
{
    fn into_response(self) -> Response {
        match self {
            either::Either::Left(l) => l.into_response(),
            either::Either::Right(r) => r.into_response(),
        }
    }
}

impl<L, R> Reply for futures::future::Either<L, R>
where
    L: Reply,
    R: Reply,
{
    fn into_response(self) -> Response {
        match self {
            futures::future::Either::Left(l) => l.into_response(),
            futures::future::Either::Right(r) => r.into_response(),
        }
    }
}

pub fn reply() -> impl Reply {
    StatusCode::OK
}

impl Reply for () {
    fn into_response(self) -> Response {
        reply().into_response()
    }
}

#[derive(Clone)]
pub struct WithStatus<R: Reply> {
    reply: R,
    status: StatusCode,
}

pub fn with_status<R: Reply>(reply: R, status: StatusCode) -> WithStatus<R> {
    WithStatus { reply, status }
}

impl<R: Reply> Reply for WithStatus<R> {
    #[inline]
    fn into_response(self) -> Response {
        let mut res = self.reply.into_response();

        // Don't override server errors with non-server errors
        if !(res.status().is_server_error() && !self.status.is_server_error()) {
            *res.status_mut() = self.status;
        }

        res
    }
}

pub struct WithHeader<R: Reply, H: Header> {
    reply: R,
    header: H,
}

impl<R: Reply, H: Header> Reply for WithHeader<R, H> {
    #[inline]
    fn into_response(self) -> Response {
        let mut res = self.reply.into_response();
        res.headers_mut().typed_insert(self.header);
        res
    }
}

impl Reply for &'static str {
    #[inline]
    fn into_response(self) -> Response {
        Response::new(Body::from(self))
    }
}

impl Reply for String {
    #[inline]
    fn into_response(self) -> Response {
        Response::new(Body::from(self))
    }
}

impl Reply for Body {
    #[inline]
    fn into_response(self) -> Response {
        Response::new(self)
    }
}

impl Reply for Response {
    #[inline]
    fn into_response(self) -> Response {
        self
    }
}

impl Reply for StatusCode {
    #[inline]
    fn into_response(self) -> Response {
        let mut res = Response::new(Body::empty());
        *res.status_mut() = self;
        res
    }
}

impl ReplyError for StatusCode {
    #[inline]
    fn status(&self) -> StatusCode {
        *self
    }

    #[inline]
    fn into_error_response(self) -> Response {
        self.into_response()
    }
}

impl ReplyError for Response {
    #[inline]
    fn status(&self) -> StatusCode {
        self.status()
    }

    #[inline]
    fn into_error_response(self) -> Response {
        self
    }
}
