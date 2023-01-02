use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicBool, AtomicI64, Ordering},
        Arc,
    },
};

use config::Config;
use filesystem::store::FileStore;
use futures::StreamExt;
use schema::Snowflake;
use tokio::sync::{Mutex, OwnedMutexGuard, Semaphore};
use util::cmap::CHashMap;

use crate::{
    backend::{
        cache::permission_cache::PermissionCache, cache::session_cache::SessionCache, gateway::Gateway,
        queues::Queues, services::Services,
    },
    Error,
};

use crate::web::{file_cache::MainFileCache, rate_limit::RateLimitTable};

pub mod emoji;
pub mod id_lock;

pub struct InnerServerState {
    pub db: db::DatabasePools,
    pub config: Config,
    pub id_lock: id_lock::IdLockMap,
    /// Each permit represents 1 Kibibyte
    pub mem_semaphore: Semaphore,
    pub cpu_semaphore: Semaphore,
    pub fs_semaphore: Semaphore,
    pub perm_cache: PermissionCache,
    pub session_cache: SessionCache,
    pub services: Services,
    pub queues: Queues,
    pub gateway: Gateway,
    pub rate_limit: RateLimitTable,
    pub file_cache: MainFileCache,
    pub emoji: self::emoji::EmojiMap,
    pub hasher: ahash::RandomState,
}

#[derive(Clone)]
pub struct ServerState(Arc<InnerServerState>);

impl Deref for ServerState {
    type Target = InnerServerState;

    fn deref(&self) -> &InnerServerState {
        &self.0
    }
}

impl ServerState {
    pub fn new(config: Config, db: db::DatabasePools) -> Self {
        ServerState(Arc::new(InnerServerState {
            db,
            id_lock: Default::default(),
            mem_semaphore: Semaphore::new(config.general.memory_limit as usize),
            cpu_semaphore: Semaphore::new(config.general.cpu_limit as usize),
            fs_semaphore: Semaphore::new(1024),
            perm_cache: PermissionCache::new(),
            session_cache: SessionCache::default(),
            services: Services::start().expect("Services failed to start correctly"),
            queues: Queues::default(),
            gateway: Gateway::default(),
            rate_limit: RateLimitTable::default(),
            file_cache: MainFileCache::default(),
            emoji: Default::default(),
            hasher: ahash::RandomState::new(),
            config,
        }))
    }

    pub fn fs(&self) -> FileStore {
        FileStore {
            root: &self.config.paths.data_path,
        }
    }
}

impl ServerState {
    pub async fn refresh_emojis(&self) -> Result<(), Error> {
        let db = self.db.read.get().await?;

        mod q {
            pub use schema::*;
            pub use thorn::*;

            thorn::indexed_columns! {
                pub enum EmojisColumns {
                    Emojis::Id,
                    Emojis::Emoji,
                }
            }
        }

        let stream = db
            .query_stream_cached_typed(
                || {
                    use q::*;
                    Query::select().cols(EmojisColumns::default()).from_table::<Emojis>()
                },
                &[],
            )
            .await?;

        futures::pin_mut!(stream);

        use self::emoji::EmojiEntry;
        use q::EmojisColumns;

        let mut emojis = Vec::new();

        while let Some(row) = stream.next().await {
            let row = row?;

            emojis.push(EmojiEntry {
                id: row.try_get(EmojisColumns::id())?,
                emoji: row.try_get(EmojisColumns::emoji())?,
                ..EmojiEntry::default()
            });
        }

        self.emoji.refresh(emojis);

        Ok(())
    }
}
