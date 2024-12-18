use std::{
    ops::Deref,
    sync::atomic::{AtomicBool, AtomicU32},
};
use tokio::sync::{mpsc, Notify};
use triomphe::Arc;

use crate::prelude::*;

use super::{ConnectionId, Event, Heart};

pub struct GatewayConnectionInner {
    pub id: ConnectionId,
    pub is_active: AtomicBool,
    pub kill: Notify,
    pub heart: Arc<Heart>,
    pub last_heartbeat: AtomicU32,
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

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl GatewayConnection {
    pub fn new(state: &GatewayServerState) -> (Self, mpsc::Receiver<Event>) {
        let heart = state.gateway.heart.clone();
        let (tx, rx) = mpsc::channel(16);
        let conn = GatewayConnection(Arc::new(GatewayConnectionInner {
            id: state.sf.gen(),
            kill: Notify::new(),
            is_active: AtomicBool::new(false),
            last_heartbeat: AtomicU32::new(heart.now()),
            heart,
            tx,
        }));

        (conn, rx)
    }

    pub fn heartbeat(&self) {
        self.last_heartbeat.fetch_max(self.heart.now(), std::sync::atomic::Ordering::Relaxed);
    }
}
