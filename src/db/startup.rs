use std::str::FromStr;
use std::sync::{atomic::Ordering, Arc};
use std::{borrow::Cow, path::PathBuf};

use futures::StreamExt;

use tokio::sync::mpsc;

use tokio_postgres as pg;

use super::client::Client;

pub fn log_connection(client: Client) {
    tokio::spawn(async move {
        loop {
            let mut conn = client.conn.lock().await;
            if let Some(ref mut rx) = *conn {
                while let Some(msg) = rx.recv().await {
                    // TODO
                }
            }

            if !client.autoreconnect.load(Ordering::SeqCst) {
                break;
            }
        }
    });
}

pub async fn startup() -> anyhow::Result<Client> {
    let db_str =
        std::env::var("DB_STR").unwrap_or_else(|_| "postgresql://user:password@db:5432".to_owned());

    let mut config = db_str.parse::<pg::Config>()?;

    config.dbname("postgres");
    let mut client = Client::connect(config.clone()).await?;

    // Just log any errors and ignore other things
    log_connection(client.clone());

    log::info!("Querying database setup...");
    let has_lantern = client
        .query_opt("SELECT 1 FROM pg_database WHERE datname=$1", &[&"lantern"])
        .await?;

    if has_lantern.is_none() {
        client
            .execute("CREATE DATABASE lantern ENCODING = 'UTF8'", &[])
            .await?;
    }

    client.close().await;

    config.dbname("lantern");
    client = Client::connect(config.clone()).await?;

    // Just log any errors and ignore other things
    log_connection(client.clone());

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
        let name = migration_path.file_stem().unwrap().to_string_lossy();
        if idx > last_migration {
            log::info!("Running migration {}: {}", idx, name);

            let migration = load_migration(migration_path).await?;

            async fn run_batch(client: &Client, sql: &str) -> Result<(), anyhow::Error> {
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
        } else {
            log::info!("Skipping migration {}: {}", idx, name);
        }
    }

    client.close().await;

    // reconnect one last time to clear old rx forwards
    // TODO: Find a way to close this on drop
    Client::connect(config).await
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

#[cfg(test)]
mod tests {
    use super::*;

    static QUERY_A: &str = r#"

CREATE TABLE lantern.users (
    --- Snowflake id
    id              bigint              NOT NULL,
    deleted_at      timestamp,
    username        varchar(64)         NOT NULL,
    -- 2-byte integer that can be displayed as 4 hex digits
    discriminator   smallint            NOT NULL,
    email           text                NOT NULL,
    dob             date                NOT NULL,
    is_verified     bool                NOT NULL    DEFAULT false,
    passhash        text                NOT NULL,
    nickname        varchar(256),
    -- custom_status tracks the little blurb that appears on users
    custom_status   varchar(128),
    -- biography is an extended user description on their profile
    biography       varchar(1024),
    -- this is for client-side user preferences, which can be stored as JSON easily enough
    preferences     jsonb,

    -- 0/NULL for online, 1 for away, 2 for busy, 3 for invisible
    away            smallint,

    CONSTRAINT users_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.users OWNER TO postgres;

-- Fast lookup of users with identical usernames
CREATE INDEX CONCURRENTLY user_username_idx ON lantern.users
    USING hash (username);

-- Fast lookup of users via `username#0000`
CREATE INDEX CONCURRENTLY user_username_discriminator_idx ON lantern.users
    USING btree (username, discriminator);

CREATE TABLE lantern.users_freelist (
    username        varchar(64) NOT NULL,
    descriminator   smallint    NOT NULL
);
ALTER TABLE lantern.users_freelist OWNER TO postgres;

CREATE INDEX CONCURRENTLY user_freelist_username_idx ON lantern.users_freelist
    USING hash (username);

    "#;

    static QUERY_B: &str = r#"
    CREATE OR REPLACE PROCEDURE lantern.register_user(
        _id bigint,
        _username varchar(64),
        _email text,
        _passhash text,
        _dob date
     )
     LANGUAGE plpgsql AS
     $$
     DECLARE
        _discriminator smallint;
     BEGIN
         SELECT descriminator INTO _discriminator FROM lantern.users_freelist WHERE username = _username;

         IF FOUND THEN
             DELETE FROM lantern.users_freelist WHERE username = _username AND discriminator = _discriminator;
         ELSE
             SELECT discriminator INTO _discriminator FROM lantern.users WHERE username = _username ORDER BY id DESC;

             IF NOT FOUND THEN
                 _discriminator := 0;
             ELSIF _discriminator = -1 THEN
                 RAISE EXCEPTION 'Username % exhausted', _username;
             ELSE
                 _discriminator := _discriminator + 1;
             END IF;
         END IF;

         INSERT INTO lantern.users (id, username, discriminator, email, passhash, dob) VALUES (_id, _username, _discriminator, _email, _passhash, _dob);
     END
     $$;

    "#;

    #[test]
    fn test_split_a() {
        for part in SqlIterator::new(QUERY_A).take(20) {
            println!("{};", part);
            println!("------------------------------");
        }
    }
}
