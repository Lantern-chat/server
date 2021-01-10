use std::str::FromStr;
use std::sync::Arc;
use std::{borrow::Cow, path::PathBuf};

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
        .query_opt("SELECT 1 FROM pg_database WHERE datname=$1", &[&"lantern"])
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

    let has_migrations = client
        .query_opt(
            "SELECT 1 FROM pg_tables WHERE schemaname = $1 AND tablename = $2",
            &[&"lantern", &"host"],
        )
        .await?;

    let mut last_migration: i32 = -1;

    if has_migrations.is_some() {
        let newest_migration = client
            .query_one(
                "SELECT (migration) FROM lantern.host ORDER BY migration DESC LIMIT 1",
                &[],
            )
            .await?;

        last_migration = newest_migration.get(0);
    }

    log::info!("Last migration: {}", last_migration);

    let mut available_migrations = std::fs::read_dir("./backend/sql/migrations")?
        .map(|res| {
            res.map_err(anyhow::Error::from).and_then(|e| {
                let path = e.path();
                let s = path.file_stem().unwrap().to_string_lossy();

                let (idx, _) = s.char_indices().find(|(i, c)| !c.is_ascii_digit()).unwrap();

                let migration_number = i32::from_str(&s[..idx])?;

                Ok((migration_number, path))
            })
        })
        .collect::<Result<Vec<_>, anyhow::Error>>()?;

    available_migrations.sort_by_key(|(key, _)| *key);

    let comment_regex = regex::Regex::new(r#"--.*"#).unwrap();

    for (idx, migration_path) in available_migrations {
        if idx > last_migration {
            log::info!("Running migration {}", idx);

            let migration = load_migration(migration_path).await?;

            async fn run_batch(client: &Client, sql: &str) -> Result<(), anyhow::Error> {
                let commands = str::split(&sql, ";");

                for command in commands {
                    client.execute(command, &[]).await?;
                }

                Ok(())
            }

            if let Err(e) = run_batch(&client, &migration.up).await {
                log::error!("Migration error: {}", e);
                log::warn!("Rolling back migration {}...", idx);

                run_batch(&client, &migration.down).await?;

                return Err(e);
            }

            client
                .execute(
                    "INSERT INTO lantern.host (migration, migrated) VALUES ($1, now());",
                    &[&idx],
                )
                .await?;
        } else {
            log::info!("Skipping migration {}", idx);
        }
    }

    // TODO: Setup migration tables and begin migrations

    Ok(()) // TODO: Return client and conn_stream
}

pub struct Migration {
    up: String,
    down: String,
}

lazy_static::lazy_static! {
    static ref SQL_COMMENT: regex::Regex = regex::Regex::new(r#"--.*"#).unwrap();
}

async fn load_sql(path: PathBuf) -> Result<String, anyhow::Error> {
    let mut sql = tokio::fs::read_to_string(path).await?;

    sql = SQL_COMMENT.replace_all(&sql, "").into_owned();

    Ok(sql)
}

async fn load_migration(path: PathBuf) -> Result<Migration, anyhow::Error> {
    let mut up = path.clone();
    let mut down = path;

    up.push("up.sql");
    down.push("down.sql");

    Ok(Migration {
        up: load_sql(up).await?,
        down: load_sql(down).await?,
    })
}
