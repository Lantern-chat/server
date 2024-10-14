#![cfg_attr(not(debug_assertions), allow(unused_mut, unused_variables, unused_imports))]
#![allow(clippy::redundant_pattern_matching, clippy::identity_op, clippy::redundant_closure)]
#![deny(deprecated)]

#[macro_use]
extern crate serde;

extern crate tracing as log;

pub mod allocator;
pub mod asset;
pub mod cli;
pub mod config;
pub mod error;
pub mod gateway;
pub mod internal;
pub mod queues;
pub mod rpc;
pub mod services;
pub mod state;
pub mod tasks;
pub mod util;

pub mod prelude {
    pub use crate::error::Error;
    pub use crate::state::ServerState;

    pub use futures::stream::{Stream, StreamExt};

    pub use rpc::{auth::Authorization, event::ServerEvent, DeserializeExt};
    pub use sdk::models::{aliases::*, Nullable, SmolStr, Snowflake, Timestamp};

    pub type ConnectionId = Snowflake;
    pub type EmbedId = Snowflake;

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

    let db = {
        use db::{Pool, PoolConfig};

        let pool_config = local.db.db_str.parse::<PoolConfig>()?;

        db::DatabasePools {
            write: Pool::new(pool_config.clone(), db::pg::NoTls),
            read: Pool::new(pool_config.readonly(), db::pg::NoTls),
        }
    };

    let shared = {
        let db = db.read.get().await?;

        schema::config::SharedConfig::load(&db).await?
    };

    let state = state::ServerState::new(config::Config { local, shared }, db);

    log::info!("Running startup task...");
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

    runner.wait().await?;

    println!("Flushing logs...");
    drop(_log_guard);
    println!("Goodbye.");

    Ok(())
}

pub mod built {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}
