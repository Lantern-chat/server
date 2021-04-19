use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::SystemTime,
};

use crate::db::Snowflake;

use super::Event;

use tokio::sync::{broadcast, mpsc, RwLock};

pub struct GatewayConnectionInner {
    //pub user_id: Snowflake,
    pub id: Snowflake,
    pub is_active: AtomicBool,
    pub last_msg: RwLock<SystemTime>,
    pub tx: mpsc::Sender<Event>,
}

#[derive(Clone)]
pub struct GatewayConnection(Arc<GatewayConnectionInner>);

impl Deref for GatewayConnection {
    type Target = GatewayConnectionInner;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl GatewayConnection {
    pub fn new() -> (Self, mpsc::Receiver<Event>) {
        let (tx, rx) = mpsc::channel(1);
        let now = SystemTime::now();
        let conn = GatewayConnection(Arc::new(GatewayConnectionInner {
            id: Snowflake::at(now),
            is_active: AtomicBool::new(false),
            last_msg: RwLock::new(now),
            tx,
        }));

        (conn, rx)
    }

    pub async fn heartbeat(&self) {
        *self.last_msg.write().await = SystemTime::now();
    }
}
