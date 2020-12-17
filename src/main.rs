pub mod db;
pub mod routes;
pub mod server;
pub mod state;

use slog::Drain;
use std::sync::{Arc, Mutex};

use warp::Filter;

#[tokio::main]
async fn main() {
    //let root = slog::Logger::root(
    //    Mutex::new(slog_json::Json::default(std::io::stderr())).map(slog::Fuse),
    //    slog::o!("version" => env!("CARGO_PKG_VERSION")),
    //);

    server::start_server(Arc::new(state::ServerState {})).await;
}
