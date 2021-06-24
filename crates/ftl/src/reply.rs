use headers::{ContentType, Header, HeaderMapExt};
use http::{Response as HttpResponse, StatusCode};
use hyper::Body;

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

pub struct MsgPack {
    inner: Result<Bytes, ()>,
}

pub fn try_msgpack<T: serde::Serialize>(value: &T, named: bool) -> Result<MsgPack, rmp_serde::encode::Error> {
    let res = match named {
        true => rmp_serde::to_vec_named(value),
        false => rmp_serde::to_vec(value),
    };

    Ok(MsgPack {
        inner: match res {
            Ok(v) => Ok(Bytes::from(v)),
            Err(e) => return Err(e),
        },
    })
}

pub fn msgpack<T: serde::Serialize>(value: &T, named: bool) -> MsgPack {
    match try_msgpack(value, named) {
        Ok(resp) => resp,
        Err(e) => {
            log::error!("MsgPack Reply error: {}", e);
            MsgPack { inner: Err(()) }
        }
    }
}

impl Reply for MsgPack {
    fn into_response(self) -> Response {
        match self.inner {
            Ok(body) => Body::from(body)
                .with_header(ContentType::from(mime::APPLICATION_MSGPACK))
                .into_response(),
            Err(()) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Clone)]
pub struct Json {
    inner: Result<Bytes, ()>,
}

pub fn try_json<T: serde::Serialize>(value: &T) -> Result<Json, serde_json::Error> {
    Ok(Json {
        inner: match serde_json::to_vec(value) {
            Ok(v) => Ok(Bytes::from(v)),
            Err(e) => return Err(e),
        },
    })
}

pub fn json<T: serde::Serialize>(value: &T) -> Json {
    match try_json(value) {
        Ok(resp) => resp,
        Err(e) => {
            log::error!("JSON Reply error: {}", e);
            Json { inner: Err(()) }
        }
    }
}

impl Reply for Json {
    fn into_response(self) -> Response {
        match self.inner {
            Ok(body) => Body::from(body).with_header(ContentType::json()).into_response(),

            Err(()) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

use bytes::Bytes;
use futures::{Stream, StreamExt};

pub struct JsonStream {
    body: Body,
}

pub fn json_stream<T, E>(stream: impl Stream<Item = Result<T, E>> + Send + 'static) -> impl Reply
where
    T: serde::Serialize + Send + Sync + 'static,
    E: Into<Box<dyn std::error::Error + Send + Sync>> + Send + Sync + 'static,
{
    let (mut sender, body) = Body::channel();

    tokio::spawn(async move {
        futures::pin_mut!(stream);

        let mut first = true;
        let mut buffer = Vec::with_capacity(128);
        buffer.push(b'[');

        let error: Result<(), Box<dyn std::error::Error + Send + Sync>> = loop {
            match stream.next().await {
                Some(Ok(ref value)) => {
                    let pos = buffer.len();

                    if !first {
                        buffer.push(b',');
                    }

                    if let Err(e) = serde_json::to_writer(&mut buffer, value) {
                        buffer.truncate(pos); // revert back to previous element
                        break Err(e.into());
                    }

                    first = false;
                }
                Some(Err(e)) => break Err(e.into()),
                None => break Ok(()),
            }

            // Flush buffer at 4KB
            if buffer.len() >= 4096 {
                let chunk = Bytes::from(std::mem::replace(&mut buffer, Vec::new()));
                if let Err(e) = sender.send_data(chunk).await {
                    log::error!("Error sending JSON array chunk: {}", e);
                    return sender.abort();
                }
            }
        };

        buffer.push(b']');
        if let Err(e) = sender.send_data(buffer.into()).await {
            log::error!("Error sending JSON array chunk: {}", e);
            return sender.abort();
        }

        if let Err(e) = error {
            log::error!("Error serializing json array: {}", e);
        }
    });

    JsonStream { body }
}

impl Reply for JsonStream {
    fn into_response(self) -> Response {
        self.body.with_header(ContentType::json()).into_response()
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
