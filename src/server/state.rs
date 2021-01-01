use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use futures::FutureExt;
use tokio::sync::{oneshot, Mutex, RwLock};

use hashbrown::HashMap;

use super::{conns::HostConnections, rate::RateLimitTable};

pub struct ServerState {
    pub is_alive: AtomicBool,
    pub shutdown: Mutex<Option<oneshot::Sender<()>>>,
    pub rate_limit: crate::server::rate::RateLimitTable,
    pub gateway_conns: HostConnections,
}

impl ServerState {
    pub fn new(shutdown: oneshot::Sender<()>) -> Arc<Self> {
        Arc::new(ServerState {
            is_alive: AtomicBool::new(true),
            shutdown: Mutex::new(Some(shutdown)),
            rate_limit: RateLimitTable::new(50.0),
            gateway_conns: HostConnections::default(),
        })
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
