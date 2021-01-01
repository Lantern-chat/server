use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use futures::FutureExt;

pub mod auth;
pub mod gateway;
pub mod rate;
pub mod routes;
pub mod state;
pub mod subs;

pub use state::ServerState;

pub mod tasks {
    pub mod cn_cleanup;
    pub mod rl_cleanup;
}

pub fn start_server(addr: impl Into<SocketAddr>) -> (impl Future<Output = ()>, Arc<ServerState>) {
    let (snd, rcv) = tokio::sync::oneshot::channel();
    let state = state::ServerState::new(snd);
    let addr = addr.into();

    log::info!("Binding to {:?}", addr);

    let (_, server) = warp::serve(routes::routes(state.clone()))
        .bind_with_graceful_shutdown(addr, rcv.map(|_| { /* ignore errors */ }));

    log::info!("Starting tasks...");

    tokio::spawn(tasks::rl_cleanup::cleanup_ratelimits(state.clone()));
    tokio::spawn(tasks::cn_cleanup::cleanup_connections(state.clone()));

    (server, state)
}
