use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::{util::SqlIterator, Client, Error, Pool};

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

    log::info!("Last migration: {last_migration}");

    let mut available_migrations = std::fs::read_dir(path)?
        .map(|res| {
            res.map_err(MigrationError::from).map(|e| {
                let path = e.path();
                let s = path.file_stem().unwrap().to_string_lossy();

                let (idx, _) = s.char_indices().find(|(_, c)| !c.is_ascii_digit()).unwrap();

                let migration_number = i32::from_str(&s[..idx]).unwrap();

                (migration_number, path)
            })
        })
        .collect::<Result<Vec<_>, MigrationError>>()?;

    available_migrations.sort_by_key(|(key, _)| *key);

    for (idx, migration_path) in available_migrations {
        let name = migration_path.file_stem().unwrap().to_string_lossy();
        if idx <= last_migration {
            log::info!("Skipping migration {idx}: {name}");
        } else {
            log::info!("Running migration {idx}: {name}");

            let migration = load_migration(migration_path).await?;

            async fn run_batch(client: &Client, sql: &str) -> Result<(), MigrationError> {
                for command in SqlIterator::new(sql) {
                    client.execute(command, &[]).await?;
                }

                Ok(())
            }

            if let Err(e) = run_batch(&client, &migration.up).await {
                log::error!("Migration error: {e}");
                log::warn!("Rolling back migration {idx}...");

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

async fn load_migration(path: PathBuf) -> Result<Migration, MigrationError> {
    let mut up = path.clone();
    let mut down = path;

    up.push("up.sql");
    down.push("down.sql");

    Ok(Migration {
        up: tokio::fs::read_to_string(up).await?,
        down: tokio::fs::read_to_string(down).await?,
    })
}
