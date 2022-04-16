extern crate tracing as log;
use db::pg::NoTls;
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    FmtSubscriber,
};

pub mod allocator;
pub mod cli;

use futures::FutureExt;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let args = cli::CliOptions::parse()?;
    log::debug!("Arguments: {:?}", args);

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

    let db = {
        use db::pool::{Pool, PoolConfig};

        let db_str =
            std::env::var("DB_STR").unwrap_or_else(|_| "postgresql://user:password@db:5432".to_owned());

        let migrations_path =
            std::env::var("MIGRATIONS").unwrap_or_else(|_| "./backend/sql/migrations".to_owned());

        let mut config = db_str.parse::<db::pg::Config>()?;
        config.dbname("lantern");

        let pool_config = PoolConfig::new(config);

        let write_pool = Pool::new(pool_config.clone(), NoTls);

        db::migrate::migrate(write_pool.clone(), migrations_path).await?;

        server::DatabasePools {
            write: write_pool,
            read: Pool::new(pool_config.readonly(), NoTls),
        }
    };

    log::info!("Starting server...");
    let (server, state) = server::start_server(args.bind, db).await?;

    log::trace!("Setting up shutdown signal for Ctrl+C");
    tokio::spawn(tokio::signal::ctrl_c().then(move |_| async move { state.shutdown().await }));

    server.await?;

    Ok(())
}
