use std::{convert::Infallible, future::Future, net::SocketAddr};

use futures::FutureExt;

use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Server,
};

pub mod body;
pub mod fs;
pub mod rate_limit;
pub mod reply;
pub mod routes;
pub mod service;
pub mod state;
pub mod util;

pub mod tasks {
    pub mod cn_cleanup;
    pub mod rl_cleanup;
    pub mod session_cleanup;
}

pub use state::ServerState;

use crate::db::Client;

pub fn start_server(
    addr: SocketAddr,
    db: Client,
) -> (impl Future<Output = Result<(), hyper::Error>>, ServerState) {
    let (snd, rcv) = tokio::sync::oneshot::channel();
    let state = ServerState::new(snd, db);

    let inner_state = state.clone();
    let server = Server::bind(&addr)
        .http2_adaptive_window(true)
        .serve(make_service_fn(move |socket: &AddrStream| {
            let remote_addr = socket.remote_addr();
            let state = inner_state.clone();

            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    service::service(remote_addr, req, state.clone())
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
