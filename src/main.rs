#![allow(unused)]

#[macro_use]
extern crate serde;

extern crate tracing as log;
use tracing_subscriber::FmtSubscriber;

pub mod built;
pub mod cli;
pub mod db;
pub mod util;
pub mod server;

use std::{net::SocketAddr, str::FromStr, sync::Arc};

use futures::FutureExt;
use structopt::StructOpt;
use warp::Filter;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let args = cli::CliOptions::from_args();

    // a builder for `FmtSubscriber`.
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(match args.verbose {
            None | Some(0) => log::Level::INFO,
            Some(1) => log::Level::DEBUG,
            _ => log::Level::TRACE,
        })
        .finish(); // completes the builder.

    log::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    tracing_log::LogTracer::init()?;

    log::debug!("Arguments: {:?}", args);

    log::trace!("Parsing bind address...");
    let addr = match args.bind {
        Some(addr) => SocketAddr::from_str(&addr)?,
        None => SocketAddr::from(([127, 0, 0, 1], 3030)),
    };

    log::info!("Starting server...");
    let (server, state) = server::start_server(addr);

    log::trace!("Setting up shutdown signal for Ctrl+C");
    tokio::spawn(tokio::signal::ctrl_c().then(move |_| async move { state.shutdown().await }));

    let _ = tokio::spawn(server).await;

    Ok(())
}
