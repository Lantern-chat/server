use std::borrow::Cow;

use futures::StreamExt;

use tokio_postgres as pg;

use super::{client::Client, conn::ConnectionStream};

pub async fn connect(config: pg::Config) -> Result<Client, Box<dyn std::error::Error>> {
    log::info!(
        "Connecting to database {:?} at {:?}:{:?}...",
        config.get_dbname(),
        config.get_hosts(),
        config.get_ports(),
    );
    let (client, conn) = config.connect(pg::NoTls).await?;

    tokio::spawn(async move {
        let mut conn = ConnectionStream(conn);
        while let Some(msg) = conn.next().await {
            match msg {
                Ok(msg) => log::info!("{:?}", msg),
                Err(e) => log::error!("Connection error: {}", e),
            }
        }
        log::info!("Disconnected from database {:?}", config.get_dbname());
    });

    Ok(Client::new(client))
}

pub async fn startup() -> Result<(), Box<dyn std::error::Error>> {
    let db_str =
        std::env::var("DB_STR").unwrap_or_else(|_| "postgresql://user:password@db:5432".to_owned());

    let mut config = db_str.parse::<pg::Config>()?;

    config.dbname("postgres");
    let mut client = connect(config.clone()).await?;

    log::info!("Querying database setup...");
    let has_lantern = client
        .query_opt(
            "SELECT (datname) FROM pg_database WHERE datname=$1",
            &[&"lantern"],
        )
        .await?;

    if has_lantern.is_none() {
        client
            .execute(
                r#"
            CREATE DATABASE lantern ENCODING = 'UTF8';
                "#,
                &[],
            )
            .await?;
    }

    config.dbname("lantern");
    client = connect(config).await?;

    // TODO: Setup migration tables and begin migrations

    Ok(())
}
