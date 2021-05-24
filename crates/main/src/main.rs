extern crate tracing as log;
use db::pg::NoTls;
use tracing_subscriber::FmtSubscriber;

pub mod cli;

use std::{net::SocketAddr, str::FromStr};

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

    let db = {
        use db::pool::{Pool, PoolConfig};

        let db_str = std::env::var("DB_STR")
            .unwrap_or_else(|_| "postgresql://user:password@db:5432".to_owned());

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
    let (server, state) = server::start_server(addr, db);

    log::trace!("Setting up shutdown signal for Ctrl+C");
    tokio::spawn(tokio::signal::ctrl_c().then(move |_| async move { state.shutdown().await }));

    server.await?;

    Ok(())
}
