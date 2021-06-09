use std::{any::Any, time::Instant};

use db::Snowflake;
use util::cmap::CHashMap;

pub struct EventItemCache {
    max_len: usize,
    pub map: CHashMap<Snowflake, (Instant, Box<dyn Any + Send + Sync>)>,
}

impl Default for EventItemCache {
    fn default() -> Self {
        Self::new(1024 * 5)
    }
}

impl EventItemCache {
    pub fn new(max_len: usize) -> Self {
        EventItemCache {
            max_len,
            map: CHashMap::default(),
        }
    }

    /// NOTE: This is not guaranteed to add to the cache
    pub async fn maybe_add<T>(&self, key: Snowflake, value: T)
    where
        T: Any + Send + Sync,
    {
        if self.map.len() < self.max_len {
            self.map.insert(key, (Instant::now(), Box::new(value))).await;
        }
    }

    /// NOTE: If you do not give the correct type, the cached value will be dropped
    pub async fn try_take<T>(&self, key: Snowflake) -> Result<Option<Box<T>>, ()>
    where
        T: Any + Send + Sync,
    {
        match self.map.remove(&key).await {
            Some((_, value)) => match value.downcast() {
                Ok(value) => Ok(Some(value)),
                Err(_) => Err(()),
            },
            None => Ok(None),
        }
    }
}
