use std::{convert::Infallible, path::PathBuf, time::Duration};
use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use futures::FutureExt;
use tokio::sync::{oneshot, Mutex, RwLock, Semaphore};

use crate::{config::LanternConfig, db::Client, fs::disk::FileStore};

use super::{ftl::rate_limit::RateLimitTable, gateway::Gateway};

pub struct InnerServerState {
    pub is_alive: AtomicBool,
    pub shutdown: Mutex<Option<oneshot::Sender<()>>>,
    pub rate_limit: RateLimitTable,
    pub db: Client,
    pub config: LanternConfig,
    pub fs: FileStore,
    pub gateway: Gateway,
    pub hashing_semaphore: Semaphore,
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
    pub fn new(shutdown: oneshot::Sender<()>, db: Client) -> Self {
        ServerState(Arc::new(InnerServerState {
            is_alive: AtomicBool::new(true),
            shutdown: Mutex::new(Some(shutdown)),
            rate_limit: RateLimitTable::new(),
            //gateway_conns: HostConnections::default(),
            db,
            config: Default::default(),   // TODO: Load from file
            fs: FileStore::new("./data"), // TODO: Set from config
            gateway: Gateway::default(),
            hashing_semaphore: Semaphore::new(16), // TODO: Set from available memory?
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

                self.db.read.close().await;
                self.db.write.close().await;

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
}
