use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use futures::FutureExt;

pub mod routes;
pub mod state;

pub use state::ServerState;

pub fn start_server(addr: impl Into<SocketAddr>) -> (impl Future<Output = ()>, Arc<ServerState>) {
    let (snd, rcv) = tokio::sync::oneshot::channel();
    let state = state::ServerState::new(snd);
    let addr = addr.into();

    log::info!("Binding to {:?}", addr);

    let (_, server) = warp::serve(routes::routes(state.clone()))
        .bind_with_graceful_shutdown(addr, rcv.map(|_| { /* ignore errors */ }));

    (server, state)
}
