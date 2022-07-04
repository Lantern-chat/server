use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicBool, AtomicI64, Ordering},
        Arc,
    },
};

use config::Config;
use filesystem::store::FileStore;
use schema::Snowflake;
use tokio::sync::{Mutex, OwnedMutexGuard, Semaphore};
use util::cmap::CHashMap;

use crate::backend::{
    cache::permission_cache::PermissionCache, cache::session_cache::SessionCache, db::DatabasePools,
    gateway::Gateway, queues::Queues, services::Services,
};

use crate::web::{file_cache::MainFileCache, rate_limit::RateLimitTable};

pub struct InnerServerState {
    pub db: DatabasePools,
    pub config: Config,
    pub id_lock: IdLockMap,
    /// Each permit represents 1 Kibibyte of the limit
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
}

#[derive(Clone)]
pub struct ServerState(Arc<InnerServerState>);

impl Deref for ServerState {
    type Target = InnerServerState;

    fn deref(&self) -> &InnerServerState {
        &*self.0
    }
}

impl ServerState {
    pub fn new(config: Config, db: DatabasePools) -> Self {
        ServerState(Arc::new(InnerServerState {
            db,
            id_lock: IdLockMap::default(),
            // memory_limit is defined in mebibytes, so convert it to kibibytes
            mem_semaphore: Semaphore::new((config.general.memory_limit / 1024) as usize),
            cpu_semaphore: Semaphore::new(num_cpus::get() * 3 / 2),
            fs_semaphore: Semaphore::new(1024),
            perm_cache: PermissionCache::new(),
            session_cache: SessionCache::default(),
            services: Services::start().expect("Services failed to start correctly"),
            queues: Queues::default(),
            gateway: Gateway::default(),
            rate_limit: RateLimitTable::new(),
            file_cache: MainFileCache::default(),
            config,
        }))
    }

    pub fn fs(&self) -> FileStore {
        FileStore {
            root: &self.config.paths.data_path,
        }
    }
}

/// Simple concurrent map structure containing locks for any particular snowflake ID
#[derive(Default, Debug)]
pub struct IdLockMap {
    pub map: CHashMap<Snowflake, Arc<Mutex<()>>>,
}

impl IdLockMap {
    pub async fn lock(&self, id: Snowflake) -> OwnedMutexGuard<()> {
        let lock = self.map.get_or_default(&id).await.clone();
        Mutex::lock_owned(lock).await
    }

    pub async fn cleanup(&self) {
        self.map.retain(|_, lock| Arc::strong_count(lock) > 1).await
    }
}
