use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use futures::FutureExt;
use tokio::sync::{oneshot, Mutex, RwLock};

use hashbrown::HashMap;

use super::rate::RateLimitTable;

pub struct ServerState {
    pub is_alive: AtomicBool,
    pub shutdown: Mutex<Option<oneshot::Sender<()>>>,
    pub rate_limit: crate::server::rate::RateLimitTable,
}

impl ServerState {
    pub fn new(shutdown: oneshot::Sender<()>) -> Arc<Self> {
        let fresh_state = Arc::new(ServerState {
            is_alive: AtomicBool::new(true),
            shutdown: Mutex::new(Some(shutdown)),
            rate_limit: RateLimitTable::new(50.0),
        });

        let state = fresh_state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            while state.is_alive.load(Ordering::Relaxed) {
                let now = interval.tick().await;
                state.rate_limit.cleanup_at(now.into_std()).await;
            }
        });

        fresh_state
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
