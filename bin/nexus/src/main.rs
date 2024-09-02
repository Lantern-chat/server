#![cfg_attr(not(debug_assertions), allow(unused_mut, unused_variables, unused_imports))]
#![allow(clippy::redundant_pattern_matching, clippy::identity_op, clippy::redundant_closure)]
#![deny(deprecated)]

#[macro_use]
extern crate serde;

extern crate tracing as log;

pub mod allocator;
pub mod api;
pub mod asset;
pub mod cli;
pub mod config;
pub mod error;
pub mod gateway;
pub mod queues;
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

    dotenv::dotenv()?;

    let mut config = config::LocalConfig::default();
    ::config::Configuration::configure(&mut config);

    #[inline(never)]
    fn get<T>() -> T {
        unimplemented!()
    }

    _ = gateway::rpc::dispatch(get(), tokio::io::empty(), get()).await;

    Ok(())
}

pub mod built {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

/*
use cli::CliOptions;
use db::{pg::NoTls, DatabasePools};
use std::sync::Arc;

pub mod allocator;
pub mod cli;

use server::config::{Config, ConfigError};
use task_runner::TaskRunner;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let args = CliOptions::parse()?;

    // temporary logger until info needed for global logger is loaded
    let (dispatch, _) = common::logging::generate(args.verbose, None)?;
    let _log_guard = log::dispatcher::set_default(&dispatch);

    log::debug!("Arguments: {:?}", args);

    let config = load_config(&args).await?;

    if args.write_config {
        log::info!("Saving config to: {}", args.config.display());
        config.save(&args.config).await?;
        return Ok(());
    }

    // setup full logger
    log::info!("Setting up log-file rotation in {}", config.paths.log_dir.display());
    drop(_log_guard);
    let (dispatch, _log_guard) = logging::generate(args.verbose, Some(config.paths.log_dir.clone()))?;
    log::dispatcher::set_global_default(dispatch).expect("setting default subscriber failed");

    let db = {
        use db::{Pool, PoolConfig};

        let pool_config = config.db.db_str.parse::<PoolConfig>()?;

        DatabasePools {
            write: Pool::new(pool_config.clone(), NoTls),
            read: Pool::new(pool_config.readonly(), NoTls),
        }
    };

    let state = server::ServerState::new(config, db);

    log::info!("Running startup tasks...");
    server::tasks::startup::run_startup_tasks(&state).await;

    log::info!("Starting tasks...");
    let runner = TaskRunner::default();
    server::tasks::add_tasks(&state, &runner);

    log::trace!("Setting up shutdown signal for Ctrl+C");
    let shutdown = runner.signal();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        shutdown.stop();
    });

    let s1 = state.clone();
    tokio::spawn(async move {
        log::debug!("Waiting for config reload signal");

        loop {
            s1.config_reload.notified().await;

            log::info!("Reloading config");

            match load_config(&args).await {
                Err(e) => log::error!("Error loading config: {e}"),
                Ok(config) => {
                    use db::PoolConfig;

                    let db_config = match config.db.db_str.parse::<PoolConfig>() {
                        Ok(c) => c,
                        Err(e) => {
                            log::error!("Error parsing database config: {e}");
                            continue;
                        }
                    };

                    s1.db.write.replace_config(db_config.clone());
                    s1.db.read.replace_config(db_config.readonly());

                    s1.set_config(Arc::new(config))
                }
            }
        }
    });

    #[cfg(unix)]
    {
        let s2 = state.clone();
        tokio::spawn(async move {
            loop {
                log::debug!("Waiting for SIGUSR1 signal...");

                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::user_defined1())
                    .unwrap()
                    .recv()
                    .await;

                log::info!("SIGUSR1 received");
                s2.trigger_config_reload();
            }
        });
    }

    runner.wait().await?;

    println!("Flushing logs to file...");
    drop(_log_guard);
    println!("Goodbye.");

    Ok(())
}
*/
