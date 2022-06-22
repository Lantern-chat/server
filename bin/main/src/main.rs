extern crate tracing as log;
use db::pg::NoTls;
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    FmtSubscriber,
};

pub mod allocator;
pub mod cli;

use task_runner::TaskRunner;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let args = cli::CliOptions::parse()?;

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

    use server::config::{Config, ConfigError};

    log::info!("Loading config from: {}", args.config_path.display());
    let mut config = match Config::load(&args.config_path).await {
        Ok(config) => config,
        Err(ConfigError::IOError(e)) if e.kind() == std::io::ErrorKind::NotFound && args.write_config => {
            log::warn!("Config file not found, but --write-config given, therefore assuming defaults");

            Config::default()
        }
        Err(e) => return Err(e.into()),
    };

    log::info!("Applying environment overrides");
    config.apply_overrides();

    if args.write_config {
        // if write-config requested, do this before saving
        config.configure();

        log::info!("Saving config to: {}", args.config_path.display());
        config.save(&args.config_path).await?;

        return Ok(());
    }

    config.configure();

    let db = {
        use db::pool::{Pool, PoolConfig};

        let mut db_config = config.db.db_str.parse::<db::pg::Config>()?;
        db_config.dbname("lantern");

        let pool_config = PoolConfig::new(db_config);

        let write_pool = Pool::new(pool_config.clone(), NoTls);

        db::migrate::migrate(write_pool.clone(), &config.db.migrations).await?;

        server::backend::db::DatabasePools {
            write: write_pool,
            read: Pool::new(pool_config.readonly(), NoTls),
        }
    };

    let state = server::ServerState::new(config, db);

    log::info!("Starting tasks...");
    let runner = TaskRunner::new();
    server::tasks::add_tasks(&state, &runner);

    log::trace!("Setting up shutdown signal for Ctrl+C");
    let shutdown = runner.signal();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        shutdown.stop();
    });

    runner.wait().await?;

    Ok(())
}
