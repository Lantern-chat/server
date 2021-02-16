use std::borrow::Borrow;
use std::hash::{BuildHasher, Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use hashbrown::hash_map::{DefaultHashBuilder, HashMap, RawEntryMut};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Debug)]
pub struct CHashMap<K, T, S = DefaultHashBuilder> {
    hash_builder: S,
    shards: Vec<RwLock<HashMap<K, T, S>>>,
}

impl<K, T> CHashMap<K, T, DefaultHashBuilder> {
    pub fn new(num_shards: usize) -> Self {
        Self::with_hasher(num_shards, DefaultHashBuilder::new())
    }
}

impl<K, T> Default for CHashMap<K, T, DefaultHashBuilder> {
    fn default() -> Self {
        Self::new(128)
    }
}

/// Simple sharded hashmap using Tokio async rwlocks for the shards
///
/// Use as a simple replacement for `RwLock<HashMap<K, T, V>>`
impl<K, T, S> CHashMap<K, T, S>
where
    S: Clone,
{
    pub fn with_hasher(num_shards: usize, hash_builder: S) -> Self {
        CHashMap {
            shards: (0..num_shards)
                .into_iter()
                .map(|_| RwLock::new(HashMap::with_hasher(hash_builder.clone())))
                .collect(),
            hash_builder,
        }
    }
}

pub struct ReadValue<'a, K, T, S> {
    lock: RwLockReadGuard<'a, HashMap<K, T, S>>,
    value: &'a T,
}

pub struct WriteValue<'a, K, T, S> {
    lock: RwLockWriteGuard<'a, HashMap<K, T, S>>,
    value: &'a mut T,
}

impl<'a, K, T, S> WriteValue<'a, K, T, S> {
    pub fn downgrade(this: WriteValue<'a, K, T, S>) -> ReadValue<'a, K, T, S> {
        ReadValue {
            lock: RwLockWriteGuard::downgrade(this.lock),
            value: this.value,
        }
    }
}

impl<K, T, S> Deref for ReadValue<'_, K, T, S> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<K, T, S> Deref for WriteValue<'_, K, T, S> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<K, T, S> DerefMut for WriteValue<'_, K, T, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

impl<K, T, S> CHashMap<K, T, S>
where
    K: Hash + Eq,
    S: BuildHasher,
{
    pub async fn retain<F>(&self, mut f: F)
    where
        F: Fn(&K, &mut T) -> bool,
    {
        use futures::StreamExt;

        let f = &f;
        futures::stream::iter(self.shards.iter())
            .for_each_concurrent(None, |shard| async move {
                let mut shard = shard.write().await;
                shard.retain(f);
            })
            .await;
    }

    fn hash_and_shard<Q>(&self, key: &Q) -> (u64, usize)
    where
        Q: Hash + Eq,
    {
        let mut hasher = self.hash_builder.build_hasher();
        key.hash(&mut hasher);
        let hash = hasher.finish();
        let shard = hash as usize % self.shards.len();
        (hash, shard)
    }

    pub async fn get_cloned<Q>(&self, key: &Q) -> Option<T>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
        T: Clone,
    {
        let (hash, shard_idx) = self.hash_and_shard(key);
        let shard = self.shards[shard_idx].read().await;
        shard
            .raw_entry()
            .from_key_hashed_nocheck(hash, key)
            .map(|(_, value)| value.clone())
    }

    pub async fn get<Q>(&self, key: &Q) -> Option<ReadValue<'_, K, T, S>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let (hash, shard_idx) = self.hash_and_shard(key);

        let shard = self.shards[shard_idx].read().await;

        match shard.raw_entry().from_key_hashed_nocheck(hash, key) {
            Some((_, value)) => Some(ReadValue {
                // cast lifetime, but it's fine because we own it while the lock is valid
                value: unsafe { std::mem::transmute(value) },
                lock: shard,
            }),
            None => None,
        }
    }

    pub async fn get_mut<Q>(&self, key: &Q) -> Option<WriteValue<'_, K, T, S>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let (hash, shard_idx) = self.hash_and_shard(key);

        let mut shard = self.shards[shard_idx].write().await;

        match shard.raw_entry_mut().from_key_hashed_nocheck(hash, key) {
            RawEntryMut::Occupied(mut entry) => Some(WriteValue {
                // cast lifetime, but it's fine because we own it while the lock is valid
                value: unsafe { std::mem::transmute(entry.get_mut()) },
                lock: shard,
            }),
            _ => None,
        }
    }

    pub async fn insert(&self, key: K, value: T) -> Option<T> {
        let (hash, shard_idx) = self.hash_and_shard(&key);
        self.shards[shard_idx].write().await.insert(key, value)
    }

    pub async fn get_or_insert(
        &self,
        key: &K,
        on_insert: impl FnOnce() -> T,
    ) -> ReadValue<'_, K, T, S>
    where
        K: Clone,
    {
        let (hash, shard_idx) = self.hash_and_shard(key);

        let mut shard = self.shards[shard_idx].write().await;

        let (_, value) = shard
            .raw_entry_mut()
            .from_key_hashed_nocheck(hash, key)
            .or_insert_with(|| (key.clone(), on_insert()));

        ReadValue {
            value: unsafe { std::mem::transmute(value) },
            lock: RwLockWriteGuard::downgrade(shard),
        }
    }

    pub async fn get_mut_or_insert(
        &self,
        key: &K,
        on_insert: impl FnOnce() -> T,
    ) -> WriteValue<'_, K, T, S>
    where
        K: Clone,
    {
        let (hash, shard_idx) = self.hash_and_shard(key);

        let mut shard = self.shards[shard_idx].write().await;

        let (_, value) = shard
            .raw_entry_mut()
            .from_key_hashed_nocheck(hash, key)
            .or_insert_with(|| (key.clone(), on_insert()));

        WriteValue {
            value: unsafe { std::mem::transmute(value) },
            lock: shard,
        }
    }

    pub async fn get_or_default(&self, key: &K) -> ReadValue<'_, K, T, S>
    where
        K: Clone,
        T: Default,
    {
        self.get_or_insert(key, Default::default).await
    }

    pub async fn get_mut_or_default(&self, key: &K) -> WriteValue<'_, K, T, S>
    where
        K: Clone,
        T: Default,
    {
        self.get_mut_or_insert(key, Default::default).await
    }
}
