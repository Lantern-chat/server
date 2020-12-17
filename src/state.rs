use std::sync::Arc;

use futures::FutureExt;
use tokio::sync::{oneshot, Mutex, RwLock};

use hashbrown::HashMap;
use warp::hyper::Server;

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
                log::debug!("Sending server shutdown signal");

                if let Err(e) = shutdown.send(()) {
                    tokio::spawn(
                        tokio::time::sleep(std::time::Duration::from_secs(5))
                            .map(|_| std::process::exit(1)),
                    );
                }
            }
            _ => {}
        }
    }
}
