use std::{
    any::Any,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};

use db::Snowflake;
use futures::{future::BoxFuture, FutureExt};
use tokio::sync::{oneshot, Mutex, Notify, Semaphore};
use util::cmap::CHashMap;

use crate::{config::LanternConfig, filesystem::disk::FileStore, DatabasePools};
use crate::{
    tasks::events::cache::EventItemCache,
    web::{gateway::Gateway, ip_bans::IpBans, rate_limit::RateLimitTable},
};

pub struct InnerServerState {
    pub is_alive: AtomicBool,
    pub notify_shutdown: Arc<Notify>,
    pub shutdown: Mutex<Option<oneshot::Sender<()>>>,
    pub rate_limit: RateLimitTable,
    pub db: DatabasePools,
    pub config: LanternConfig,
    pub fs: FileStore,
    pub gateway: Gateway,
    pub hashing_semaphore: Semaphore,
    pub all_tasks: Mutex<
        Option<
            BoxFuture<'static, Result<Result<(), tokio::task::JoinError>, tokio::task::JoinError>>,
        >,
    >,
    pub item_cache: EventItemCache,
    pub ip_bans: IpBans,
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
    pub fn new(shutdown: oneshot::Sender<()>, db: DatabasePools) -> Self {
        ServerState(Arc::new(InnerServerState {
            is_alive: AtomicBool::new(true),
            notify_shutdown: Arc::new(Notify::new()),
            shutdown: Mutex::new(Some(shutdown)),
            rate_limit: RateLimitTable::new(),
            //gateway_conns: HostConnections::default(),
            db,
            config: Default::default(),   // TODO: Load from file
            fs: FileStore::new("./data"), // TODO: Set from config
            gateway: Gateway::default(),
            hashing_semaphore: Semaphore::new(16), // TODO: Set from available memory?
            all_tasks: Mutex::new(None),
            item_cache: EventItemCache::default(),
            ip_bans: IpBans::new(),
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

                self.db.read.close().await;
                self.db.write.close().await;

                if let Some(all_tasks) = self.all_tasks.lock().await.take() {
                    match all_tasks.await {
                        Ok(Ok(_)) => log::info!("Tasks ended successfully!"),
                        Err(e) | Ok(Err(e)) => log::error!("Tasks errored on shutdown: {}", e),
                    }
                }

                if let Err(err) = shutdown.send(()) {
                    log::error!("Could not shutdown server gracefully! Error: {:?}", err);
                    log::error!("Forcing process exit in 5 seconds!");

                    tokio::spawn(
                        tokio::time::sleep(std::time::Duration::from_secs(5))
                            .map(|_| std::process::exit(1)),
                    );
                }
            }
            None => log::warn!("Duplicate shutdown signals detected!"),
        }
    }

    pub async fn read_db(&self) -> db::pool::Object {
        self.db
            .read
            .get()
            .await
            .expect("Could not acquire readonly database connection")
    }

    pub async fn write_db(&self) -> db::pool::Object {
        self.db
            .write
            .get()
            .await
            .expect("Could not acquire writable database connection")
    }
}
