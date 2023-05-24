use std::{ops::Deref, sync::atomic::AtomicBool, time::Instant};
use tokio::sync::{mpsc, Notify, RwLock};
use triomphe::Arc;

use schema::{Snowflake, SnowflakeExt};

use super::Event;

pub struct GatewayConnectionInner {
    pub id: Snowflake,
    pub is_active: AtomicBool,
    pub kill: Notify,
    pub last_msg: RwLock<Instant>,
    pub tx: mpsc::Sender<Event>,
}

#[cfg(debug_assertions)]
impl Drop for GatewayConnectionInner {
    fn drop(&mut self) {
        log::debug!("Dropping connection {}", self.id);
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct GatewayConnection(Arc<GatewayConnectionInner>);

impl Deref for GatewayConnection {
    type Target = GatewayConnectionInner;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl GatewayConnection {
    pub fn new() -> (Self, mpsc::Receiver<Event>) {
        let (tx, rx) = mpsc::channel(1);
        let conn = GatewayConnection(Arc::new(GatewayConnectionInner {
            id: Snowflake::now(),
            kill: Notify::new(),
            is_active: AtomicBool::new(false),
            last_msg: RwLock::new(Instant::now()),
            tx,
        }));

        (conn, rx)
    }

    pub async fn heartbeat(&self) {
        *self.last_msg.write().await = Instant::now();
    }
}
