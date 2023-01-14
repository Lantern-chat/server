extern crate tracing as log;
use cli::CliOptions;
use db::{pg::NoTls, DatabasePools};
use std::sync::Arc;
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    FmtSubscriber,
};

pub mod allocator;
pub mod cli;

use server::config::{Config, ConfigError};
use task_runner::TaskRunner;

async fn load_config(args: &CliOptions) -> anyhow::Result<Config> {
    log::info!("Loading config from: {}", args.config_path.display());
    let mut config = match Config::load(&args.config_path).await {
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
    dotenv::dotenv().ok();

    let args = CliOptions::parse()?;

    let mut extreme_trace = false;

    let level_filter = match args.verbose {
        None | Some(0) => LevelFilter::INFO,
        Some(1) => LevelFilter::DEBUG,
        Some(2) => LevelFilter::TRACE,
        Some(3) | _ => {
            extreme_trace = true;
            LevelFilter::TRACE
        }
    };

    // a builder for `FmtSubscriber`.
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_env_filter({
            let filter = EnvFilter::from_default_env()
                .add_directive(level_filter.into())
                .add_directive("hyper::client::pool=info".parse()?)
                .add_directive("hyper::proto=info".parse()?)
                .add_directive("tokio_util::codec=info".parse()?);

            if !extreme_trace {
                filter.add_directive("server::tasks=debug".parse()?)
            } else {
                filter
            }
        })
        .finish(); // completes the builder.

    log::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    log::debug!("Arguments: {:?}", args);

    let config = load_config(&args).await?;

    if args.write_config {
        log::info!("Saving config to: {}", args.config_path.display());
        config.save(&args.config_path).await?;

        return Ok(());
    }

    let db = {
        use db::pool::{Pool, PoolConfig};

        let pool_config = config.db.db_str.parse::<PoolConfig>()?;

        let write_pool = Pool::new(pool_config.clone(), NoTls);

        //db::migrate::migrate(write_pool.clone(), &config.db.migrations).await?;

        DatabasePools {
            write: write_pool,
            read: Pool::new(pool_config.readonly(), NoTls),
        }
    };

    let state = server::ServerState::new(config, db);

    log::info!("Running startup tasks...");
    server::tasks::startup::run_startup_tasks(&state).await;

    log::info!("Starting tasks...");
    let runner = TaskRunner::new();
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

    Ok(())
}
