use std::{
    any::Any,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, AtomicI64, Ordering},
        Arc,
    },
    time::Instant,
};

use config::Config;
use futures::{future::BoxFuture, FutureExt};
use schema::Snowflake;
use tokio::sync::{oneshot, Mutex, Notify, OwnedMutexGuard, Semaphore};
use util::cmap::CHashMap;

use crate::{
    cache::permission_cache::PermissionCache, cache::session_cache::SessionCache, db::DatabasePools,
    queues::Queues, services::Services,
};

pub struct InnerState {
    pub is_alive: AtomicBool,
    pub notify_shutdown: Arc<Notify>,
    pub shutdown: Mutex<Option<oneshot::Sender<()>>>,
    pub db: DatabasePools,
    pub config: Config,
    pub id_lock: IdLockMap,
    pub hashing_semaphore: Semaphore,
    pub processing_semaphore: Semaphore,
    pub perm_cache: PermissionCache,
    pub session_cache: SessionCache,
    pub totp_tokens: TokenStorage,
    pub services: Services,
    pub queues: Queues,
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
    pub fn new(shutdown: oneshot::Sender<()>, config: Config, db: DatabasePools) -> Self {
        State(Arc::new(InnerState {
            is_alive: AtomicBool::new(true),
            notify_shutdown: Arc::new(Notify::new()),
            shutdown: Mutex::new(Some(shutdown)),
            db,
            config,
            id_lock: IdLockMap::default(),
            hashing_semaphore: Semaphore::new(16), // TODO: Set from available memory?
            processing_semaphore: Semaphore::new(num_cpus::get() * 2),
            perm_cache: PermissionCache::new(),
            session_cache: SessionCache::default(),
            totp_tokens: TokenStorage::default(),
            services: Services::start().expect("Services failed to start correctly"),
            queues: Queues::default(),
        }))
    }

    #[inline]
    pub fn is_alive(&self) -> bool {
        self.is_alive.load(Ordering::Relaxed)
    }

    pub async fn shutdown(&self) {
        match self.shutdown.lock().await.take() {
            Some(shutdown) => {
                log::info!("Sending server shutdown signal.");

                self.is_alive.store(false, Ordering::Relaxed);
                self.notify_shutdown.notify_waiters();
                self.queues.stop();

                self.db.read.close().await;
                self.db.write.close().await;

                //if let Some(all_tasks) = self.all_tasks.lock().await.take() {
                //    match all_tasks.await {
                //        Ok(Ok(_)) => log::info!("Tasks ended successfully!"),
                //        Err(e) | Ok(Err(e)) => log::error!("Tasks errored on shutdown: {e}"),
                //    }
                //}

                if let Err(err) = shutdown.send(()) {
                    log::error!("Could not shutdown server gracefully! Error: {:?}", err);
                    log::error!("Forcing process exit in 5 seconds!");

                    tokio::spawn(
                        tokio::time::sleep(std::time::Duration::from_secs(5)).map(|_| std::process::exit(1)),
                    );
                }
            }
            None => log::warn!("Duplicate shutdown signals detected!"),
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

#[derive(Default)]
pub struct TokenStorage {
    pub map: CHashMap<Snowflake, (Instant, Arc<[u8]>)>,
}

impl TokenStorage {
    pub async fn add(&self, id: Snowflake, token: impl AsRef<[u8]>) {
        self.map
            .insert(id, (Instant::now(), Arc::from(token.as_ref())))
            .await;
    }

    pub async fn get(&self, id: Snowflake) -> Option<Arc<[u8]>> {
        self.map.get_cloned(&id).await.map(|(_, token)| token)
    }

    pub async fn remove(&self, id: Snowflake) {
        self.map.remove(&id).await;
    }
}
