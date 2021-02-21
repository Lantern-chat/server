use std::{convert::Infallible, net::SocketAddr, str::Split};

use bytes::Buf;
use http::Method;
use hyper::{
    body::{aggregate, HttpBody},
    Body, Request, Response,
};

use crate::server::ServerState;

pub struct Route {
    pub addr: SocketAddr,
    pub req: Request<Body>,
    pub state: ServerState,
    pub segment_index: usize,
    pub has_body: bool,
}

impl Route {
    pub fn tail(&self) -> &str {
        &self.req.uri().path()[self.segment_index..]
    }

    pub fn next_segment(&mut self) -> &str {
        self.next_segment_method().1
    }

    pub fn next_segment_method(&mut self) -> (&Method, &str) {
        let path = self.req.uri().path();

        let segment = path[self.segment_index..]
            .split('/') // split the next segment
            .next() // only take the first
            .expect("split always has at least 1");

        if !segment.is_empty() {
            let index = self.segment_index + segment.len();

            // if already at the end, don't increment
            self.segment_index = if path.len() == index {
                index
            } else {
                // otherwise skip the slash
                debug_assert_eq!(path.as_bytes()[index], b'/');
                index + 1
            };
        }

        (self.req.method(), segment)
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

    pub async fn aggregate(&mut self) -> Result<impl Buf, BodyError> {
        Ok(match self.take_body() {
            Some(body) => hyper::body::aggregate(body).await?,
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
