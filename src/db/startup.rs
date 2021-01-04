use std::borrow::Cow;
use std::sync::Arc;

use futures::StreamExt;

use tokio::sync::mpsc;

use tokio_postgres as pg;

use super::{client::Client, conn::ConnectionStream};

pub async fn connect(
    config: pg::Config,
) -> anyhow::Result<(Client, mpsc::UnboundedReceiver<pg::AsyncMessage>)> {
    log::info!(
        "Connecting to database {:?} at {:?}:{:?}...",
        config.get_dbname(),
        config.get_hosts(),
        config.get_ports(),
    );
    let (client, conn) = config.connect(pg::NoTls).await?;

    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        let mut conn = ConnectionStream(conn);
        while let Some(msg) = conn.next().await {
            match msg {
                Ok(msg) => {
                    if let Err(e) = tx.send(msg) {
                        log::error!("Error forwarding database event: {:?}", e);
                    }
                }
                Err(e) => {
                    log::error!("Database error: {:?}", e);
                    break;
                }
            }
        }
        log::info!("Disconnected from database {:?}", config.get_dbname());
    });

    Ok((Client::new(client), rx))
}

pub async fn startup() -> anyhow::Result<()> {
    let db_str =
        std::env::var("DB_STR").unwrap_or_else(|_| "postgresql://user:password@db:5432".to_owned());

    let mut config = db_str.parse::<pg::Config>()?;

    config.dbname("postgres");
    let (mut client, mut conn_stream) = connect(config.clone()).await?;

    // While we're determining database setup, just log any errors and ignore other things
    tokio::spawn(async move {
        while let Some(msg) = conn_stream.recv().await {
            // TODO
        }
    });

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
    let (mut client, conn_stream) = connect(config).await?;

    // TODO: Setup migration tables and begin migrations

    Ok(()) // TODO: Return client and conn_stream
}
