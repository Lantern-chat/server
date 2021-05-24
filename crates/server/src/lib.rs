#![allow(unused_imports)]

#[macro_use]
extern crate serde;

extern crate tracing as log;

use std::{convert::Infallible, future::Future, net::SocketAddr};

use futures::FutureExt;

use ftl::StatusCode;
use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Server,
};

pub mod built {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub mod config;
pub mod ctrl;
pub mod filesystem;
pub mod state;
pub mod util;
pub mod web;

pub mod tasks {
    pub mod cn_cleanup;
    pub mod rl_cleanup;
    pub mod session_cleanup;
}

pub use state::ServerState;

use db::pool::Pool;

#[derive(Clone)]
pub struct DatabasePools {
    pub read: Pool,
    pub write: Pool,
}

use ftl::Reply;

pub fn start_server(
    addr: SocketAddr,
    db: DatabasePools,
) -> (impl Future<Output = Result<(), hyper::Error>>, ServerState) {
    let (snd, rcv) = tokio::sync::oneshot::channel();
    let state = ServerState::new(snd, db);

    let inner_state = state.clone();
    let server = Server::bind(&addr)
        .http2_adaptive_window(true)
        .tcp_nodelay(true)
        .serve(make_service_fn(move |socket: &AddrStream| {
            let remote_addr = socket.remote_addr();
            let state = inner_state.clone();

            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    let state = state.clone();

                    // gracefully fail and return HTTP 500
                    async move {
                        match tokio::spawn(web::service::service(remote_addr, req, state)).await {
                            Ok(resp) => resp,
                            Err(err) => {
                                log::error!("Internal Server Error: {}", err);

                                Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response())
                            }
                        }
                    }
                }))
            }
        }))
        .with_graceful_shutdown(rcv.map(|_| { /* ignore errors */ }));

    log::info!("Starting interval tasks...");

    tokio::spawn(tasks::rl_cleanup::cleanup_ratelimits(state.clone()));
    tokio::spawn(tasks::cn_cleanup::cleanup_connections(state.clone()));
    tokio::spawn(tasks::session_cleanup::cleanup_sessions(state.clone()));

    (server, state)
}
