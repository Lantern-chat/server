use headers::{ContentType, Header, HeaderMapExt};
use http::{Response as HttpResponse, StatusCode};
use hyper::Body;

pub type Response = HttpResponse<Body>;

pub trait Reply {
    fn into_response(self) -> Response;

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

pub trait ReplyError: Reply {
    fn status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
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
            Err(err) => {
                let status = err.status();
                err.with_status(status).into_response()
            }
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
    fn into_response(self) -> Response {
        match self.inner {
            Ok(body) => Body::from(body)
                .with_header(ContentType::json())
                .into_response(),

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
    fn into_response(self) -> Response {
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
}

impl ReplyError for Response {
    #[inline]
    fn status(&self) -> StatusCode {
        self.status()
    }
}
