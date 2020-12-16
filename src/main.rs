pub mod db;
pub mod routes;
pub mod state;

use slog::Drain;
use std::sync::Mutex;

#[tokio::main]
async fn main() {
    let root = slog::Logger::root(
        Mutex::new(slog_json::Json::default(std::io::stderr())).map(slog::Fuse),
        slog::o!("version" => env!("CARGO_PKG_VERSION")),
    );
}
