use std::{
    convert::Infallible,
    net::{AddrParseError, IpAddr, SocketAddr},
    str::FromStr,
    time::{Duration, Instant},
};

use bytes::{Buf, Bytes};
use futures::Stream;
use headers::{Header, HeaderMapExt, HeaderValue};
use http::{header::ToStrError, method::InvalidMethod, uri::Authority, Method};
use hyper::{
    body::{aggregate, HttpBody},
    Body, Request, Response,
};

// TODO: Make state generic
pub struct Route<S> {
    pub addr: SocketAddr,
    pub req: Request<Body>,
    pub state: S,
    pub segment_index: usize,
    pub next_segment_index: usize,
    pub has_body: bool,
    pub start: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Segment<'a> {
    Exact(&'a str),
    End,
}

impl<S> Route<S> {
    #[inline]
    pub fn new(addr: SocketAddr, req: Request<Body>, state: S) -> Route<S> {
        Route {
            start: Instant::now(),
            addr,
            req,
            state,
            segment_index: 0,
            next_segment_index: 0,
            has_body: true,
        }
    }

    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Use this at the start of a Route to override the provided HTTP Method with the value present in
    /// the `x-http-method-override` HTTP header.
    ///
    /// This is sometimes used in old browsers without support for PATCH or OPTIONS methods.
    pub fn apply_method_override(&mut self) -> Result<(), InvalidMethod> {
        if let Some(method_override) = self.req.headers().get("x-http-method-override") {
            *self.req.method_mut() = Method::from_bytes(method_override.as_bytes())?;
        }

        Ok(())
    }

    /// Attempt to discern the current host authority either by
    /// looking at the input URI or the `HOST` HTTP Header.
    ///
    /// If no consistent host authority can be found, `None` is returned.
    pub fn host(&self) -> Option<Authority> {
        let from_uri = self.req.uri().authority();

        let from_header = match self.parse_raw_header::<Authority>("host") {
            Some(Ok(Ok(host))) => Some(host),
            _ => None,
        };

        match (from_uri, from_header) {
            (None, None) => None,
            (Some(a), None) => Some(a.clone()),
            (None, Some(b)) => Some(b),
            (Some(a), Some(b)) if *a == b => Some(b),
            _ => None,
        }
    }

    /// Parse the URI query
    pub fn query<T: serde::de::DeserializeOwned>(&self) -> Option<Result<T, serde_urlencoded::de::Error>> {
        self.req.uri().query().map(serde_urlencoded::de::from_str)
    }

    pub fn path(&self) -> &str {
        self.req.uri().path()
    }

    /// Returns the remaining parts of the URI path **After** the current segment.
    pub fn tail(&self) -> &str {
        &self.path()[self.next_segment_index..]
    }

    /// Parses a URI as an arbitrary parameter using `FromStr`
    pub fn param<P: FromStr>(&self) -> Option<Result<P, P::Err>> {
        match self.segment() {
            Segment::Exact(segment) => Some(segment.parse()),
            Segment::End => None,
        }
    }

    /// Returns both the `Method` and URI `Segment` a the same time for convenient `match` statements.
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

    /// Parse a header value using `FromStr`
    #[inline]
    pub fn parse_raw_header<T: FromStr>(&self, name: &str) -> Option<Result<Result<T, T::Err>, ToStrError>> {
        self.raw_header(name)
            .map(|header| header.to_str().map(FromStr::from_str))
    }

    /// Parses the `Content-Length` header and returns the value as a `u64`,
    /// or `None` if there was not a set content length
    #[inline]
    pub fn content_length(&self) -> Option<u64> {
        self.header::<headers::ContentLength>().map(|cl| cl.0)
    }

    /// Parses the proxy chain in the `x-forwarded-for` HTTP header.
    pub fn forwarded_for<'a>(
        &'a self,
    ) -> Option<Result<impl Iterator<Item = Result<IpAddr, AddrParseError>> + 'a, ToStrError>> {
        self.req.headers().get("x-forwarded-for").map(|ff| {
            ff.to_str()
                .map(|ff| ff.split(',').map(|segment| IpAddr::from_str(segment.trim())))
        })
    }

    /// Finds the next segment in the URI path, storing the result internally for further usage.
    ///
    /// Use [`.segment()`], [`.method_segment()`] or [`param`] to parse the segment found (if any)
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

    pub fn body(&self) -> &Body {
        self.req.body()
    }

    /// Takes ownership of the request body, returning `None` if it was already consumed.
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
