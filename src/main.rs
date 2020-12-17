extern crate tracing as log;
use tracing_subscriber::FmtSubscriber;

pub mod db;
pub mod routes;
pub mod server;
pub mod state;

use std::sync::Arc;

use futures::FutureExt;

use warp::Filter;

#[tokio::main]
async fn main() {
    // a builder for `FmtSubscriber`.
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(log::Level::TRACE)
        .finish(); // completes the builder.

    log::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let addr = ([127, 0, 0, 1], 3030);

    let (snd, rcv) = tokio::sync::oneshot::channel();
    let state = state::ServerState::new(snd);
    let (addr, server) = warp::serve(routes::routes(state.clone()))
        .bind_with_graceful_shutdown(addr, rcv.map(|_| { /* ignore errors */ }));

    log::trace!("Setting up shutdown signal for Ctrl+C");
    tokio::spawn(tokio::signal::ctrl_c().then(move |_| async move { state.shutdown().await }));

    log::info!("Starting server...");
    tokio::spawn(server).await;
}
