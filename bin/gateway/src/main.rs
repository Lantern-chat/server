#![cfg_attr(not(debug_assertions), allow(unused_mut, unused_variables, unused_imports))]
#![allow(clippy::redundant_pattern_matching, clippy::identity_op, clippy::redundant_closure)]
#![deny(deprecated)]

extern crate tracing as log;

pub mod allocator;
pub mod cli;
pub mod config;
pub mod gateway;
pub mod nexus;
pub mod rpc;
pub mod state;
pub mod tasks;
pub mod web;

pub mod prelude {
    pub use crate::state::GatewayServerState;
    pub use common_web::error::Error;

    pub use rpc::{auth::Authorization, event::ServerEvent};
    pub use sdk::models::{aliases::*, Nullable, SmolStr, Snowflake, Timestamp};

    pub type EventId = sdk::Snowflake;
    pub type ConnectionId = sdk::Snowflake;

    pub use crate::config::Config;
    pub use config::HasConfig;

    pub use rkyv::Archived;
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    println!("Build time: {}", built::BUILT_TIME_UTC);

    let args = cli::CliOptions::parse()?;

    dotenv::dotenv()?;

    // temporary logger until info needed for global logger is loaded
    let (dispatch, _) = common::logging::generate(args.verbose, None)?;
    let _log_guard = log::dispatcher::set_default(&dispatch);

    log::debug!("Arguments: {:?}", args);

    let mut local = config::LocalConfig::default();
    ::config::Configuration::configure(&mut local);

    // setup full logger
    log::info!("Setting up log-file rotation in {}", local.paths.log_dir.display());
    drop(_log_guard);

    let (dispatch, _log_guard) = common::logging::generate(args.verbose, local.paths.log_dir.clone().into())?;
    log::dispatcher::set_global_default(dispatch)?;

    log::info!("Connecting to Nexus at {}", local.rpc.nexus_addr);
    let nexus = nexus::connect(&local).await?;

    log::info!("Fetching shared config from Nexus");
    let config = config::Config {
        local,
        shared: nexus::fetch_shared_config(&nexus).await?,
    };

    let state = state::GatewayServerState::new(config, nexus);

    log::info!("Running startup tasks...");
    // todo

    log::info!("Starting tasks...");
    let runner = tasks::TaskRunner::default();
    tasks::add_tasks(&state, &runner);

    log::trace!("Setting up shutdown signal for Ctrl+C");
    let shutdown = runner.signal();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        shutdown.stop();
    });

    log::info!("Gateway server started");
    runner.wait().await?;

    println!("Flushing logs...");
    drop(_log_guard);
    println!("Goodbye.");

    Ok(())
}

pub mod built {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}
