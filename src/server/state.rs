use std::sync::Arc;

use futures::FutureExt;
use tokio::sync::{oneshot, Mutex, RwLock};

use hashbrown::HashMap;

pub struct ServerState {
    pub shutdown: Mutex<Option<oneshot::Sender<()>>>,
}

impl ServerState {
    pub fn new(shutdown: oneshot::Sender<()>) -> Arc<Self> {
        Arc::new(ServerState {
            shutdown: Mutex::new(Some(shutdown)),
        })
    }

    pub async fn shutdown(&self) {
        match self.shutdown.lock().await.take() {
            Some(shutdown) => {
                log::info!("Sending server shutdown signal.");

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
