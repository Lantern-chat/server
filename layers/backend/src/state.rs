use std::{ops::Deref, sync::Arc};

use config::Config;
use schema::Snowflake;
use tokio::sync::{oneshot, Mutex, OwnedMutexGuard, Semaphore};
use util::cmap::CHashMap;

use crate::{
    cache::permission_cache::PermissionCache, cache::session_cache::SessionCache, db::DatabasePools,
    gateway::Gateway, queues::Queues, services::Services,
};

pub struct InnerState {
    pub db: Arc<DatabasePools>,
    pub config: Arc<Config>,
    pub id_lock: IdLockMap,
    pub hashing_semaphore: Semaphore,
    pub processing_semaphore: Semaphore,
    pub fs_semaphore: Semaphore,
    pub perm_cache: PermissionCache,
    pub session_cache: SessionCache,
    pub services: Services,
    pub queues: Queues,
    pub gateway: Gateway,
}

#[derive(Clone)]
pub struct State(Arc<InnerState>);

impl Deref for State {
    type Target = InnerState;

    fn deref(&self) -> &InnerState {
        &*self.0
    }
}

impl State {
    pub fn new(config: Arc<Config>, db: Arc<DatabasePools>) -> Self {
        State(Arc::new(InnerState {
            db,
            config,
            id_lock: IdLockMap::default(),
            hashing_semaphore: Semaphore::new(16), // TODO: Set from available memory?
            processing_semaphore: Semaphore::new(num_cpus::get() * 2),
            fs_semaphore: Semaphore::new(1024),
            perm_cache: PermissionCache::new(),
            session_cache: SessionCache::default(),
            services: Services::start().expect("Services failed to start correctly"),
            queues: Queues::default(),
            gateway: Gateway::default(),
        }))
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
