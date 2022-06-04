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
    filesystem::store::FileStore,
    permission_cache::PermissionCache,
    queues::Queues,
    services::Services,
    session_cache::SessionCache,
    tasks::events::cache::EventItemCache,
    web::file_cache::MainFileCache,
    web::{gateway::Gateway, rate_limit::RateLimitTable},
    DatabasePools,
};

pub struct InnerServerState {
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
    pub fn new(shutdown: oneshot::Sender<()>) -> Self {
        ServerState(Arc::new(InnerServerState {
            rate_limit: RateLimitTable::new(),
            file_cache: MainFileCache::default(),
        }))
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

                if let Some(all_tasks) = self.all_tasks.lock().await.take() {
                    match all_tasks.await {
                        Ok(Ok(_)) => log::info!("Tasks ended successfully!"),
                        Err(e) | Ok(Err(e)) => log::error!("Tasks errored on shutdown: {e}"),
                    }
                }

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
