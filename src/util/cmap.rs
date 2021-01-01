use std::borrow::Borrow;
use std::hash::{BuildHasher, Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;

use hashbrown::hash_map::{DefaultHashBuilder, HashMap};
use tokio::sync::{Mutex, MutexGuard, OwnedMutexGuard};

/// Simple sharded hashmap using Tokio async mutexes for the shards
#[derive(Debug)]
pub struct CHashMap<K, T, S = DefaultHashBuilder> {
    hash_builder: S,
    shards: Vec<Arc<Mutex<HashMap<K, T, S>>>>,
}

impl<K, T> CHashMap<K, T, DefaultHashBuilder> {
    pub fn new(num_shards: usize) -> Self {
        Self::with_hasher(num_shards, DefaultHashBuilder::new())
    }
}

impl<K, T, S> CHashMap<K, T, S>
where
    S: Clone,
{
    pub fn with_hasher(num_shards: usize, hash_builder: S) -> Self {
        CHashMap {
            shards: (0..num_shards)
                .into_iter()
                .map(|_| Arc::new(Mutex::new(HashMap::with_hasher(hash_builder.clone()))))
                .collect(),
            hash_builder,
        }
    }
}

pub struct BorrowedValue<'a, K, T, S> {
    lock: OwnedMutexGuard<HashMap<K, T, S>>,
    value: &'a T,
}

impl<K, T, S> Deref for BorrowedValue<'_, K, T, S> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<K, T, S> CHashMap<K, T, S>
where
    K: Hash + Eq,
    S: BuildHasher,
{
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
        let shard = self.shards[shard_idx].lock().await;
        shard
            .raw_entry()
            .from_key_hashed_nocheck(hash, key)
            .map(|(_, value)| value.clone())
    }

    pub async fn get<'a, Q>(&self, key: &Q) -> Option<BorrowedValue<'a, K, T, S>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
        T: 'a,
    {
        let (hash, shard_idx) = self.hash_and_shard(key);

        let shard = Mutex::lock_owned(self.shards[shard_idx].clone()).await;

        match shard.raw_entry().from_key_hashed_nocheck(hash, key) {
            Some((_, value)) => Some(BorrowedValue {
                value: unsafe { std::mem::transmute(value) }, // cast lifetime, but it's fine because we own it while the lock is valid
                lock: shard,
            }),
            None => None,
        }
    }

    pub async fn get_or_insert<'a>(
        &self,
        key: &K,
        default: impl FnOnce() -> T,
    ) -> BorrowedValue<'a, K, T, S>
    where
        K: Clone,
        T: 'a,
    {
        let (hash, shard_idx) = self.hash_and_shard(key);

        let mut shard = Mutex::lock_owned(self.shards[shard_idx].clone()).await;

        let (_, value) = shard
            .raw_entry_mut()
            .from_key_hashed_nocheck(hash, key)
            .or_insert_with(|| (key.clone(), default()));

        BorrowedValue {
            value: unsafe { std::mem::transmute(value) }, // cast lifetime, but it's fine because we own it while the lock is valid
            lock: shard,
        }
    }

    pub async fn get_or_default<'a>(&self, key: &K) -> BorrowedValue<'a, K, T, S>
    where
        K: Clone,
        T: 'a + Default,
    {
        self.get_or_insert(key, Default::default).await
    }

    pub async fn retain<F>(&self, mut f: F)
    where
        F: Fn(&K, &mut T) -> bool,
    {
        for shard in &self.shards {
            let mut shard = shard.lock().await;
            shard.retain(&f);
        }
    }
}
