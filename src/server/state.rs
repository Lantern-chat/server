use std::{convert::Infallible, time::Duration};
use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use futures::FutureExt;
use tokio::sync::{oneshot, Mutex, RwLock};

use hashbrown::HashMap;

use crate::db::Client;

use super::rate_limit::RateLimitTable;

pub struct InnerServerState {
    pub is_alive: AtomicBool,
    pub shutdown: Mutex<Option<oneshot::Sender<()>>>,
    pub rate_limit: crate::server::rate_limit::RateLimitTable,
    //pub gateway_conns: HostConnections,
    pub db: Client,
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
        }))
    }

    //pub fn inject(&self) -> impl Filter<Extract = (Self,), Error = Infallible> + Clone {
    //    let state = self.clone();
    //    warp::any().map(move || state.clone())
    //}

    #[inline]
    pub fn is_alive(&self) -> bool {
        self.is_alive.load(Ordering::Relaxed)
    }

    pub async fn shutdown(&self) {
        match self.shutdown.lock().await.take() {
            Some(shutdown) => {
                log::info!("Sending server shutdown signal.");

                self.is_alive.store(false, Ordering::Relaxed);

                self.db.close().await;

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
