use std::str::FromStr;
use std::sync::{atomic::Ordering, Arc};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use futures::{Stream, StreamExt};

use pg::Connection;
use tokio::sync::mpsc;

use crate::pool::{Client, ConnectionStream, Error, Pool};

#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] Error),

    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
}

pub async fn migrate<P: AsRef<Path>>(pool: Pool, path: P) -> Result<(), MigrationError> {
    let client = pool.get().await?;

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

    let mut available_migrations = std::fs::read_dir(path)?
        .map(|res| {
            res.map_err(MigrationError::from).and_then(|e| {
                let path = e.path();
                let s = path.file_stem().unwrap().to_string_lossy();

                let (idx, _) = s.char_indices().find(|(_, c)| !c.is_ascii_digit()).unwrap();

                let migration_number = i32::from_str(&s[..idx]).unwrap();

                Ok((migration_number, path))
            })
        })
        .collect::<Result<Vec<_>, MigrationError>>()?;

    available_migrations.sort_by_key(|(key, _)| *key);

    for (idx, migration_path) in available_migrations {
        let name = migration_path.file_stem().unwrap().to_string_lossy();
        if idx <= last_migration {
            log::info!("Skipping migration {}: {}", idx, name);
        } else {
            log::info!("Running migration {}: {}", idx, name);

            let migration = load_migration(migration_path).await?;

            async fn run_batch(client: &Client, sql: &str) -> Result<(), MigrationError> {
                for command in SqlIterator::new(&sql) {
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
        }
    }

    Ok(())
}

pub struct Migration {
    up: String,
    down: String,
}

lazy_static::lazy_static! {
    static ref SQL_COMMENT: regex::Regex = regex::Regex::new(r#"--.*"#).unwrap();
}

async fn load_sql(path: PathBuf) -> Result<String, MigrationError> {
    let mut sql = tokio::fs::read_to_string(path).await?;

    sql = SQL_COMMENT.replace_all(&sql, "").into_owned();

    Ok(sql)
}

async fn load_migration(path: PathBuf) -> Result<Migration, MigrationError> {
    let mut up = path.clone();
    let mut down = path;

    up.push("up.sql");
    down.push("down.sql");

    Ok(Migration {
        up: load_sql(up).await?,
        down: load_sql(down).await?,
    })
}

struct SqlIterator<'a> {
    sql: &'a str,
}

impl<'a> SqlIterator<'a> {
    pub fn new(sql: &'a str) -> Self {
        SqlIterator { sql }
    }
}

impl<'a> Iterator for SqlIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        let mut in_dollar = false;
        let mut ic = self.sql.char_indices().peekable();

        loop {
            if let Some((idx, c)) = ic.next() {
                if c == '$' && ic.peek().map(|(_, c)| *c == '$') == Some(true) {
                    in_dollar = !in_dollar;
                }

                if c == ';' && !in_dollar {
                    let res = Some(&self.sql[..idx]);
                    self.sql = &self.sql[idx + 1..];
                    return res;
                }
            } else {
                return None;
            }
        }
    }
}
