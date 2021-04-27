extern crate tracing as log;
use tracing_subscriber::FmtSubscriber;

pub mod cli;

use std::{net::SocketAddr, str::FromStr, sync::Arc};

use futures::FutureExt;
use structopt::StructOpt;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let mut args = cli::CliOptions::from_args();

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
    log::debug!("Arguments: {:?}", args);

    args.prepare()?;

    log::trace!("Parsing bind address...");
    let addr = match args.bind {
        Some(addr) => SocketAddr::from_str(&addr)?,
        None => SocketAddr::from(([127, 0, 0, 1], 3030)),
    };

    let db = db::Client::startup().await?;

    log::info!("Starting server...");
    let (server, state) = server::start_server(addr, db);

    log::trace!("Setting up shutdown signal for Ctrl+C");
    tokio::spawn(tokio::signal::ctrl_c().then(move |_| async move { state.shutdown().await }));

    server.await?;

    Ok(())
}
