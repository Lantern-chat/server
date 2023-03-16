use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicBool, AtomicI64, Ordering},
        Arc,
    },
};

use arc_swap::ArcSwap;
use config::Config;
use filesystem::store::FileStore;
use futures::{Stream, StreamExt};
use schema::Snowflake;
use tokio::sync::{watch, Mutex, Notify, OwnedMutexGuard, Semaphore};
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
    pub config: ArcSwap<Config>,
    /// Triggered when the config is reloaded
    pub config_change: Notify,
    /// when triggered, should reload the config file
    pub config_reload: Notify,

    pub id_lock: id_lock::IdLockMap,
    /// Each permit represents 1 Kibibyte
    ///
    /// Used to limit how many memory-intensive tasks are run at a time
    pub mem_semaphore: Semaphore,
    /// Used to limit how many CPU-intensive tasks are run at a time
    pub cpu_semaphore: Semaphore,
    /// Used to limit how many files are open at a given time
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
#[repr(transparent)]
pub struct ServerState(Arc<InnerServerState>);

impl Deref for ServerState {
    type Target = InnerServerState;

    #[inline(always)]
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
            perm_cache: PermissionCache::default(),
            session_cache: SessionCache::default(),
            services: Services::start().expect("Services failed to start correctly"),
            queues: Queues::default(),
            gateway: Gateway::default(),
            rate_limit: RateLimitTable::default(),
            file_cache: MainFileCache::default(),
            emoji: Default::default(),
            hasher: ahash::RandomState::new(),
            config: ArcSwap::from_pointee(config),
            config_change: Notify::new(),
            config_reload: Notify::new(),
        }))
    }

    pub fn trigger_config_reload(&self) {
        self.config_reload.notify_waiters();
    }

    pub fn set_config(&self, config: Arc<Config>) {
        // TODO: Modify semaphores to reflect changed config
        self.config.store(config);
        self.config_change.notify_waiters();
    }

    #[inline]
    pub fn config(&self) -> arc_swap::Guard<Arc<Config>, arc_swap::DefaultStrategy> {
        self.config.load()
    }

    /// Returns an infinite stream that yields a reference to the config only when it changes
    ///
    /// The first value returns immediately
    pub fn config_stream(&self) -> impl Stream<Item = arc_swap::Guard<Arc<Config>, arc_swap::DefaultStrategy>> {
        use futures::stream::{iter, repeat};

        // NOTE: `iter` has less overhead than `once`
        let first = iter([self.config()]);

        // TODO: Figure out how to avoid cloning on every item, maybe convert to stream::poll_fn
        let rest = repeat(self.clone()).then(|state| async move {
            state.config_change.notified().await;
            state.config()
        });

        first.chain(rest)
    }

    pub fn fs(&self) -> FileStore {
        FileStore {
            root: self.config().paths.data_path.clone(),
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

        use self::emoji::EmojiEntry;
        use q::EmojisColumns;

        let mut emojis = Vec::new();

        let mut stream = std::pin::pin!(stream);
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
