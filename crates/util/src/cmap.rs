use std::borrow::Borrow;
use std::hash::{BuildHasher, Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicUsize, Ordering};

use hashbrown::hash_map::{DefaultHashBuilder, HashMap, RawEntryMut};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub type CHashSet<K, S = DefaultHashBuilder> = CHashMap<K, (), S>;

#[derive(Debug)]
pub struct CHashMap<K, T, S = DefaultHashBuilder> {
    hash_builder: S,
    shards: Vec<RwLock<HashMap<K, T, S>>>,
    size: AtomicUsize,
}

impl<K, T> CHashMap<K, T, DefaultHashBuilder> {
    pub fn new(num_shards: usize) -> Self {
        Self::with_hasher(num_shards, DefaultHashBuilder::new())
    }
}

impl<K, T> Default for CHashMap<K, T, DefaultHashBuilder> {
    fn default() -> Self {
        lazy_static::lazy_static! {
            static ref NUM_CPUS: usize = num_cpus::get();
        }

        Self::new(32 * *NUM_CPUS)
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
            size: AtomicUsize::new(0),
        }
    }
}

pub struct ReadValue<'a, K, T, S> {
    _lock: RwLockReadGuard<'a, HashMap<K, T, S>>,
    value: &'a T,
}

pub struct WriteValue<'a, K, T, S> {
    _lock: RwLockWriteGuard<'a, HashMap<K, T, S>>,
    value: &'a mut T,
}

impl<'a, K, T, S> WriteValue<'a, K, T, S> {
    pub fn downgrade(this: WriteValue<'a, K, T, S>) -> ReadValue<'a, K, T, S> {
        ReadValue {
            _lock: RwLockWriteGuard::downgrade(this._lock),
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
    pub async fn retain<F>(&self, f: F)
    where
        F: Fn(&K, &mut T) -> bool,
    {
        for shard in &self.shards {
            shard.write().await.retain(|k, v| {
                let retained = f(k, v);
                if !retained {
                    self.size.fetch_sub(1, Ordering::Relaxed);
                }
                retained
            });
        }
    }

    pub fn shards(&self) -> &[RwLock<HashMap<K, T, S>>] {
        &self.shards
    }

    pub fn len(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    fn hash_and_shard<Q>(&self, key: &Q) -> (u64, usize)
    where
        Q: Hash + Eq,
    {
        let mut hasher = self.hash_builder.build_hasher();
        key.hash(&mut hasher);
        let hash = hasher.finish();
        (hash, hash as usize % self.shards.len())
    }

    pub async fn get_cloned<Q>(&self, key: &Q) -> Option<T>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
        T: Clone,
    {
        let (hash, shard_idx) = self.hash_and_shard(key);
        let shard = unsafe { self.shards.get_unchecked(shard_idx).read().await };
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

        let shard = unsafe { self.shards.get_unchecked(shard_idx).read().await };

        match shard.raw_entry().from_key_hashed_nocheck(hash, key) {
            Some((_, value)) => Some(ReadValue {
                // cast lifetime, but it's fine because we own it while the lock is valid
                value: unsafe { std::mem::transmute(value) },
                _lock: shard,
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

        let mut shard = unsafe { self.shards.get_unchecked(shard_idx).write().await };

        match shard.raw_entry_mut().from_key_hashed_nocheck(hash, key) {
            RawEntryMut::Occupied(mut entry) => Some(WriteValue {
                // cast lifetime, but it's fine because we own it while the lock is valid
                value: unsafe { std::mem::transmute(entry.get_mut()) },
                _lock: shard,
            }),
            _ => None,
        }
    }

    pub async fn remove<Q: ?Sized>(&self, key: &Q) -> Option<T>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let (hash, shard_idx) = self.hash_and_shard(&key);
        let mut shard = unsafe { self.shards.get_unchecked(shard_idx).write().await };

        match shard.raw_entry_mut().from_key_hashed_nocheck(hash, &key) {
            RawEntryMut::Occupied(occupied) => {
                self.size.fetch_sub(1, Ordering::Relaxed);
                Some(occupied.remove())
            }
            RawEntryMut::Vacant(_) => None,
        }
    }

    pub async fn insert(&self, key: K, value: T) -> Option<T> {
        let (hash, shard_idx) = self.hash_and_shard(&key);
        unsafe {
            let mut shard = self.shards.get_unchecked(shard_idx).write().await;

            let entry = shard.raw_entry_mut().from_key_hashed_nocheck(hash, &key);

            match entry {
                RawEntryMut::Occupied(mut occupied) => Some(occupied.insert(value)),
                RawEntryMut::Vacant(vacant) => {
                    vacant.insert_hashed_nocheck(hash, key, value);
                    self.size.fetch_add(1, Ordering::Relaxed);
                    None
                }
            }
        }
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

        let mut shard = unsafe { self.shards.get_unchecked(shard_idx).write().await };

        let (_, value) = shard
            .raw_entry_mut()
            .from_key_hashed_nocheck(hash, key)
            .or_insert_with(|| {
                self.size.fetch_add(1, Ordering::Relaxed);
                (key.clone(), on_insert())
            });

        ReadValue {
            value: unsafe { std::mem::transmute(value) },
            _lock: RwLockWriteGuard::downgrade(shard),
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

        let mut shard = unsafe { self.shards.get_unchecked(shard_idx).write().await };

        let (_, value) = shard
            .raw_entry_mut()
            .from_key_hashed_nocheck(hash, key)
            .or_insert_with(|| {
                self.size.fetch_add(1, Ordering::Relaxed);

                (key.clone(), on_insert())
            });

        WriteValue {
            value: unsafe { std::mem::transmute(value) },
            _lock: shard,
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

    fn batch_hash_and_sort<'a, Q: 'a, I>(&self, keys: I, cache: &mut Vec<(&'a Q, u64, usize)>)
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
        I: IntoIterator<Item = &'a Q>,
    {
        cache.truncate(0);

        cache.extend(keys.into_iter().map(|key| {
            let (hash, shard) = self.hash_and_shard(key);
            (key, hash, shard)
        }));

        if !cache.is_empty() {
            cache.sort_unstable_by_key(|(_, _, shard)| *shard);
        }
    }

    // TODO: Rewrite this to take unique shards into a small Vec,
    // then iterate over them concurrently to avoid blocking on any single one
    pub async fn batch_read<'a, Q: 'a, I, F>(
        &self,
        keys: I,
        cache: Option<&mut Vec<(&'a Q, u64, usize)>>,
        mut f: F,
    ) where
        K: Borrow<Q>,
        Q: Hash + Eq,
        I: IntoIterator<Item = &'a Q>,
        F: FnMut(&'a Q, Option<(&K, &T)>),
    {
        let mut own_cache = Vec::new();
        let cache = cache.unwrap_or(&mut own_cache);

        self.batch_hash_and_sort(keys, cache);

        if cache.is_empty() {
            return;
        }

        let mut i = 0;
        loop {
            let current_shard = cache[i].2;
            let shard = unsafe { self.shards.get_unchecked(current_shard).read().await };

            while cache[i].2 == current_shard {
                f(
                    cache[i].0,
                    shard
                        .raw_entry()
                        .from_key_hashed_nocheck(cache[i].1, cache[i].0),
                );
                i += 1;

                if i >= cache.len() {
                    return;
                }
            }
        }
    }

    // TODO: Same as with `batch_read`
    pub async fn batch_write<'a, Q: 'a, I, F>(
        &self,
        keys: I,
        cache: Option<&mut Vec<(&'a Q, u64, usize)>>,
        mut f: F,
    ) where
        K: Borrow<Q>,
        Q: Hash + Eq,
        I: IntoIterator<Item = &'a Q>,
        F: FnMut(&'a Q, hashbrown::hash_map::RawEntryMut<K, T, S>),
    {
        let mut own_cache = Vec::new();
        let cache = cache.unwrap_or(&mut own_cache);

        self.batch_hash_and_sort(keys, cache);

        if cache.is_empty() {
            return;
        }

        let mut i = 0;
        loop {
            let current_shard = cache[i].2;
            let mut shard = unsafe { self.shards.get_unchecked(current_shard).write().await };

            while cache[i].2 == current_shard {
                f(
                    cache[i].0,
                    shard
                        .raw_entry_mut()
                        .from_key_hashed_nocheck(cache[i].1, cache[i].0),
                );
                i += 1;

                if i >= cache.len() {
                    return;
                }
            }
        }
    }
}
