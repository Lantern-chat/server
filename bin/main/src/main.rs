extern crate tracing as log;
use db::pg::NoTls;
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    FmtSubscriber,
};

pub mod allocator;
pub mod cli;

use futures::FutureExt;

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

    // Load save, allow it to fill in defaults, then save it back
    log::info!("Loading config from: {}", args.config_path.display());
    let (initialized, mut config) = server::config::Config::load(&args.config_path).await?;

    log::info!("Applying environment overrides");
    config.apply_overrides();

    // if write-config requested, do this before saving
    if args.write_config {
        config.configure();
    }

    if initialized || args.write_config {
        log::info!("Saving config to: {}", args.config_path.display());
        config.save(&args.config_path).await?;

        log::info!("Save complete, exiting.");
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

        server::DatabasePools {
            write: write_pool,
            read: Pool::new(pool_config.readonly(), NoTls),
        }
    };

    log::info!("Starting server...");
    let (server, state) = server::start_server(config, db).await?;

    log::trace!("Setting up shutdown signal for Ctrl+C");
    tokio::spawn(tokio::signal::ctrl_c().then(move |_| async move { state.shutdown().await }));

    server.await?;

    Ok(())
}
