use std::{ops::Deref, sync::atomic::AtomicBool};
use tokio::sync::{mpsc, Notify};
use triomphe::Arc;

use schema::{Snowflake, SnowflakeExt};

use super::{Event, Heart};

pub struct GatewayConnectionInner {
    pub id: Snowflake,
    pub is_active: AtomicBool,
    pub kill: Notify,
    pub heart: Arc<Heart>,
    pub tx: mpsc::Sender<Event>,
}

#[cfg(debug_assertions)]
impl Drop for GatewayConnectionInner {
    fn drop(&mut self) {
        debug_assert!(!self.heart.beats.contains(&self.id));

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
    pub fn new(heart: Arc<Heart>) -> (Self, mpsc::Receiver<Event>) {
        let (tx, rx) = mpsc::channel(1);
        let conn = GatewayConnection(Arc::new(GatewayConnectionInner {
            id: Snowflake::now(),
            kill: Notify::new(),
            is_active: AtomicBool::new(false),
            heart,
            tx,
        }));

        (conn, rx)
    }

    pub async fn heartbeat(&self) {
        self.heart.beat(self.id).await;
    }
}
