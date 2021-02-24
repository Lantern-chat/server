use std::{convert::Infallible, net::SocketAddr, str::FromStr};

use bytes::{Buf, Bytes};
use futures::Stream;
use headers::{Header, HeaderMapExt, HeaderValue};
use http::{header::ToStrError, method::InvalidMethod, Method};
use hyper::{
    body::{aggregate, HttpBody},
    Body, Request, Response,
};

use crate::server::ServerState;

// TODO: Make state generic
pub struct Route {
    pub addr: SocketAddr,
    pub req: Request<Body>,
    pub state: ServerState,
    pub segment_index: usize,
    pub next_segment_index: usize,
    pub has_body: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Segment<'a> {
    Exact(&'a str),
    End,
}

impl Route {
    pub fn new(addr: SocketAddr, req: Request<Body>, state: ServerState) -> Route {
        Route {
            addr,
            req,
            state,
            segment_index: 0,
            next_segment_index: 0,
            has_body: true,
        }
    }

    pub fn apply_method_override(&mut self) -> Result<(), InvalidMethod> {
        if let Some(method_override) = self.req.headers().get("x-http-method-override") {
            *self.req.method_mut() = Method::from_bytes(method_override.as_bytes())?;
        }

        Ok(())
    }

    pub fn path(&self) -> &str {
        self.req.uri().path()
    }

    pub fn tail(&self) -> &str {
        &self.path()[self.next_segment_index..]
    }

    pub fn param<P: FromStr>(&self) -> Option<Result<P, P::Err>> {
        match self.segment() {
            Segment::Exact(segment) => Some(segment.parse()),
            Segment::End => None,
        }
    }

    #[inline]
    pub fn segment(&self) -> Segment {
        self.method_segment().1
    }

    #[inline]
    pub fn method(&self) -> &Method {
        self.req.method()
    }

    #[inline]
    pub fn header<H: Header>(&self) -> Option<H> {
        self.req.headers().typed_get()
    }

    #[inline]
    pub fn raw_header(&self, name: &str) -> Option<&HeaderValue> {
        self.req.headers().get(name)
    }

    #[inline]
    pub fn parse_raw_header<T: FromStr>(
        &self,
        name: &str,
    ) -> Option<Result<Result<T, T::Err>, ToStrError>> {
        self.raw_header(name)
            .map(|header| header.to_str().map(FromStr::from_str))
    }

    pub fn next(&mut self) -> &mut Self {
        self.segment_index = self.next_segment_index;

        let path = self.req.uri().path();

        // already at end, nothing to do
        if self.segment_index == path.len() {
            return self;
        }

        // skip leading slash
        if path.as_bytes()[self.segment_index] == b'/' {
            self.segment_index += 1;
        }

        let segment = path[self.segment_index..]
            .split('/') // split the next segment
            .next() // only take the first
            .expect("split always has at least 1");

        self.next_segment_index = self.segment_index + segment.len();

        self
    }

    pub fn method_segment(&self) -> (&Method, Segment) {
        let path = self.req.uri().path();
        let method = self.req.method();
        let segment = if self.segment_index == path.len() {
            Segment::End
        } else {
            Segment::Exact(&path[self.segment_index..self.next_segment_index])
        };

        (method, segment)
    }

    pub fn body(&self) -> &Body {
        self.req.body()
    }

    pub fn take_body(&mut self) -> Option<Body> {
        if self.has_body {
            let body = std::mem::replace(self.req.body_mut(), Body::empty());
            self.has_body = false;
            Some(body)
        } else {
            None
        }
    }

    /// Combines the body together in a buffered but chunked set of buffers
    ///
    /// Prefer this to `bytes()` when you don't care about random-access (i.e. using `buf.as_reader()`)
    pub async fn aggregate(&mut self) -> Result<impl Buf, BodyError> {
        Ok(match self.take_body() {
            Some(body) => hyper::body::aggregate(body).await?,
            None => return Err(BodyError::DoubleUseError),
        })
    }

    /// Concatenates the body together into a single contiguous buffer
    ///
    /// Prefer `aggregate()` when you don't care about random-access (i.e. using `buf.as_reader()`)
    pub async fn bytes(&mut self) -> Result<Bytes, BodyError> {
        Ok(match self.take_body() {
            Some(body) => hyper::body::to_bytes(body).await?,
            None => return Err(BodyError::DoubleUseError),
        })
    }

    pub fn stream(&mut self) -> Result<impl Stream<Item = Result<Bytes, hyper::Error>>, BodyError> {
        Ok(match self.take_body() {
            Some(body) => body,
            None => return Err(BodyError::DoubleUseError),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BodyError {
    #[error("Body cannot be used twice")]
    DoubleUseError,

    #[error("Error aggregating: {0}")]
    AggregateError(#[from] hyper::Error),
}
