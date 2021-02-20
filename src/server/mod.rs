use std::{convert::Infallible, future::Future, net::SocketAddr};

use futures::FutureExt;

use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Server,
};

pub mod rate;
pub mod reply;
pub mod service;
pub mod state;
//pub mod conns;
pub mod auth;
pub mod body;
pub mod fs;
pub mod routes;
pub mod util;

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

    (server, state)
}
