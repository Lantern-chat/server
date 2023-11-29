use arc_swap::ArcSwap;
use std::{ops::Deref, sync::Arc};

use crate::config::Config;
use futures::{Stream, StreamExt};
use sdk::Snowflake;
use tokio::sync::{Notify, Semaphore};

pub mod permission_cache;
use permission_cache::PermissionCache;

use common::{emoji::EmojiMap, id_lock::IdLockMap};

pub struct ServerStateInner {
    pub db: db::DatabasePools,
    pub config: ArcSwap<Config>,
    /// Triggered when the config is reloaded
    pub config_change: Notify,

    /// when triggered, should reload the config file
    pub config_reload: Notify,

    /// Generic lock for anything with a Snowflake ID
    pub id_lock: IdLockMap,

    /// Each permit represents 1 Kibibyte
    ///
    /// Used to limit how many memory-intensive tasks are run at a time
    pub mem_semaphore: Semaphore,

    /// Used to limit how many CPU-intensive tasks are run at a time
    pub cpu_semaphore: Semaphore,

    pub perm_cache: PermissionCache,

    // TODO
    // pub services: Services,
    // pub queues: Queues,

    // /// Generic lock for anything with a Snowflake ID
    // pub id_lock: id_lock::IdLockMap,
    ///
    pub hasher: ahash::RandomState,

    pub emoji: EmojiMap,

    /// Last timestep used for MFA per-user.
    pub mfa_last: scc::HashIndex<Snowflake, u64, ahash::RandomState>,
}

#[derive(Clone)]
#[repr(transparent)]
pub struct ServerState(triomphe::Arc<ServerStateInner>);

impl Deref for ServerState {
    type Target = ServerStateInner;

    #[inline(always)]
    fn deref(&self) -> &ServerStateInner {
        &self.0
    }
}

impl ServerState {
    pub fn new(config: Config, db: db::DatabasePools) -> Self {
        ServerState(triomphe::Arc::new(ServerStateInner {
            db,
            id_lock: Default::default(),
            mem_semaphore: Semaphore::new(config.general.memory_limit as usize),
            cpu_semaphore: Semaphore::new(config.general.cpu_limit as usize),
            perm_cache: PermissionCache::default(),
            emoji: Default::default(),
            hasher: ahash::RandomState::new(),
            config: ArcSwap::from_pointee(config),
            config_change: Notify::new(),
            config_reload: Notify::new(),
            mfa_last: Default::default(),
        }))
    }

    pub fn trigger_config_reload(&self) {
        self.config_reload.notify_waiters();
    }

    pub fn set_config(&self, config: Arc<Config>) {
        // TODO: Modify resource semaphores to reflect changed config limits
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
}
