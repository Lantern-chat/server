use flate2::Status;
use headers::{Header, HeaderMapExt};
use http::{Response, StatusCode};

use hyper::Body;

pub trait Reply {
    fn into_response(self) -> Response<Body>;

    #[inline]
    fn with_status(self, status: StatusCode) -> WithStatus<Self>
    where
        Self: Sized,
    {
        with_status(self, status)
    }

    #[inline]
    fn with_header<H>(self, header: H) -> WithHeader<Self, H>
    where
        Self: Sized,
        H: Header,
    {
        WithHeader {
            reply: self,
            header,
        }
    }
}

pub fn reply() -> impl Reply {
    StatusCode::OK
}

pub struct Json {
    inner: Result<Vec<u8>, ()>,
}

pub fn json<T: serde::Serialize>(value: &T) -> Json {
    Json {
        inner: serde_json::to_vec(value).map_err(|err| {
            log::error!("JSON Reply error: {}", err);
        }),
    }
}

impl Reply for Json {
    fn into_response(self) -> Response<Body> {
        match self.inner {
            Ok(body) => {
                let mut res = Response::new(body.into());
                res.headers_mut().typed_insert(headers::ContentType::json());
                res
            }
            Err(()) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub struct WithStatus<R: Reply> {
    reply: R,
    status: StatusCode,
}

pub fn with_status<R: Reply>(reply: R, status: StatusCode) -> WithStatus<R> {
    WithStatus { reply, status }
}

impl<R: Reply> Reply for WithStatus<R> {
    #[inline]
    fn into_response(self) -> Response<Body> {
        let mut res = self.reply.into_response();
        *res.status_mut() = self.status;
        res
    }
}

pub struct WithHeader<R: Reply, H: Header> {
    reply: R,
    header: H,
}

impl<R: Reply, H: Header> Reply for WithHeader<R, H> {
    #[inline]
    fn into_response(self) -> Response<Body> {
        let mut res = self.reply.into_response();
        res.headers_mut().typed_insert(self.header);
        res
    }
}

impl Reply for &'static str {
    #[inline]
    fn into_response(self) -> Response<Body> {
        Response::new(Body::from(self))
    }
}

impl Reply for String {
    #[inline]
    fn into_response(self) -> Response<Body> {
        Response::new(Body::from(self))
    }
}

impl Reply for Body {
    #[inline]
    fn into_response(self) -> Response<Body> {
        Response::new(self)
    }
}

impl Reply for Response<Body> {
    #[inline]
    fn into_response(self) -> Response<Body> {
        self
    }
}

impl Reply for StatusCode {
    #[inline]
    fn into_response(self) -> Response<Body> {
        let mut res = Response::new(Body::from(self.to_string()));
        *res.status_mut() = self;
        res
    }
}
