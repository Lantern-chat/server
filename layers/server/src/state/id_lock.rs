use schema::Snowflake;
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedMutexGuard, Semaphore};

/// Simple concurrent map structure containing locks for any particular snowflake ID
#[derive(Default, Debug)]
pub struct IdLockMap {
    pub map: scc::HashMap<Snowflake, Arc<Mutex<()>>>,
}

impl IdLockMap {
    pub async fn lock(&self, id: Snowflake) -> OwnedMutexGuard<()> {
        let lock = self.map.entry_async(id).await.or_default().get().clone();
        Mutex::lock_owned(lock).await
    }

    pub async fn cleanup(&self) {
        self.map.retain_async(|_, lock| Arc::strong_count(lock) > 1).await;
    }
}
