#[macro_use]
extern crate serde;

extern crate tracing as log;

pub mod built {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub mod allocator;
pub mod config;
pub mod state;

pub use crate::state::ServerState;

fn main() {}

/*
use cli::CliOptions;
use db::{pg::NoTls, DatabasePools};
use std::sync::Arc;

pub mod allocator;
pub mod cli;

use server::config::{Config, ConfigError};
use task_runner::TaskRunner;

async fn load_config(args: &CliOptions) -> anyhow::Result<Config> {
    log::info!("Loading config from: {}", args.config.display());
    let mut config = match Config::load(&args.config).await {
        Ok(config) => config,
        Err(ConfigError::IOError(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            if args.write_config {
                log::warn!("Config file not found, but `--write-config` given, therefore assuming defaults");

                Config::default()
            } else {
                let err = concat!(
                    "Config file not found, re-run with `--write-config` to generate default configuration\n\t",
                    "Or specify a config file path with `--config ./somewhere/config.toml`"
                );

                log::error!("{}", err);
                return Err(anyhow::format_err!("{}", err));
            }
        }
        Err(e) => return Err(e.into()),
    };

    log::info!("Applying environment overrides to configuration");
    config.apply_overrides();

    config.configure();

    Ok(config)
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    println!("Build time: {}", server::built::BUILT_TIME_UTC);

    dotenv::dotenv().ok();

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
        use db::pool::{Pool, PoolConfig};

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
                    use db::pool::PoolConfig;

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

    drop(LogDropWrapper(Some(_log_guard)));

    Ok(())
}

struct LogDropWrapper<T>(Option<T>);

impl<T> Drop for LogDropWrapper<T> {
    fn drop(&mut self) {
        println!("Flushing logs to file...");
        drop(self.0.take());
        println!("Goodbye.");
    }
}
*/
