use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use futures::FutureExt;

pub mod auth;
pub mod conns;
pub mod events;
pub mod gateway;
pub mod rate;
pub mod routes;
pub mod state;
pub mod storage;

pub use state::ServerState;

use db::Client;

pub mod tasks {
    pub mod cn_cleanup;
    pub mod rl_cleanup;
    pub mod session_cleanup;
}

pub fn start_server(
    addr: impl Into<SocketAddr>,
    db: Client,
) -> (impl Future<Output = ()>, ServerState) {
    let (snd, rcv) = tokio::sync::oneshot::channel();
    let state = state::ServerState::new(snd, db);
    let addr = addr.into();

    log::info!("Binding to {:?}", addr);
    let (_, server) = warp::serve(routes::routes(state.clone()))
        .bind_with_graceful_shutdown(addr, rcv.map(|_| { /* ignore errors */ }));

    log::info!("Starting interval tasks...");

    tokio::spawn(tasks::rl_cleanup::cleanup_ratelimits(state.clone()));
    tokio::spawn(tasks::cn_cleanup::cleanup_connections(state.clone()));
    tokio::spawn(tasks::session_cleanup::cleanup_sessions(state.clone()));

    (server, state)
}
