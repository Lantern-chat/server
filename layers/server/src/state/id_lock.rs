use schema::Snowflake;
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedMutexGuard, Semaphore};
use util::cmap::CHashMap;

/// Simple concurrent map structure containing locks for any particular snowflake ID
#[derive(Default, Debug)]
pub struct IdLockMap {
    pub map: CHashMap<Snowflake, Arc<Mutex<()>>>,
}

impl IdLockMap {
    pub async fn lock(&self, id: Snowflake) -> OwnedMutexGuard<()> {
        let lock = self.map.get_or_default(&id).await.clone();
        Mutex::lock_owned(lock).await
    }

    pub async fn cleanup(&self) {
        self.map.retain(|_, lock| Arc::strong_count(lock) > 1).await
    }
}
