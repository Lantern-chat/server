use std::{convert::Infallible, net::SocketAddr, str::Split};

use hyper::{Body, Request, Response};

use super::{routes::routes, ServerState};

pub struct Route {
    pub addr: SocketAddr,
    pub req: Request<Body>,
    pub state: ServerState,
    pub segment_index: usize,
}

impl Route {
    pub fn tail(&self) -> &str {
        &self.req.uri().path()[self.segment_index..]
    }

    pub fn next_segment(&mut self) -> &str {
        let path = self.req.uri().path();

        let segment = path[self.segment_index..]
            .splitn(2, '/') // split between the new segment and the rest of the path
            .next()
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

        segment
    }
}

pub async fn service(
    addr: SocketAddr,
    req: Request<Body>,
    state: ServerState,
) -> Result<Response<Body>, Infallible> {
    // skip leading slashes
    let segment_index = req.uri().path().chars().take_while(|c| *c == '/').count();

    println!("{}", segment_index);

    let resp = routes(Route {
        addr,
        req,
        state,
        segment_index,
    })
    .await;

    Ok(resp)
}
