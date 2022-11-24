use std::{
    hash::BuildHasher,
    sync::{
        atomic::{AtomicIsize, Ordering},
        Arc,
    },
};

use hashbrown::HashMap;

use util::cmap::{CHashMap, DefaultHashBuilder, ReadValue, WriteValue};

#[derive(Default, Clone)]
struct SharedBuildHasher<S: BuildHasher>(Arc<S>);

impl<S: BuildHasher> BuildHasher for SharedBuildHasher<S> {
    type Hasher = <S as BuildHasher>::Hasher;

    fn build_hasher(&self) -> Self::Hasher {
        self.0.build_hasher()
    }
}

use schema::Snowflake;

use sdk::models::Permission;

type UserId = Snowflake;
type RoomId = Snowflake;

type SHB = SharedBuildHasher<DefaultHashBuilder>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermMute {
    pub perm: Permission,
    pub muted: bool,
}

impl std::ops::Deref for PermMute {
    type Target = Permission;

    #[inline]
    fn deref(&self) -> &Permission {
        &self.perm
    }
}

// TODO: Maybe add per-party caching as well?
struct UserCache {
    room: HashMap<RoomId, PermMute, SHB>,
    rc: AtomicIsize,
}

pub struct PermissionCache {
    map: CHashMap<UserId, UserCache, SHB>,
}

impl PermissionCache {
    pub fn new() -> Self {
        PermissionCache {
            map: CHashMap::with_hasher(
                CHashMap::<(), ()>::default_num_shards(),
                SharedBuildHasher(Arc::new(DefaultHashBuilder::default())),
            ),
        }
    }

    pub async fn has_user(&self, user_id: Snowflake) -> bool {
        self.map.contains(&user_id).await
    }

    pub async fn get(&self, user_id: Snowflake, room_id: Snowflake) -> Option<PermMute> {
        self.map.get(&user_id).await.and_then(|cache| {
            // double-check if not stale
            if cache.rc.load(Ordering::Acquire) > 0 {
                cache.room.get(&room_id).copied()
            } else {
                None
            }
        })
    }

    #[inline]
    async fn get_cache(&self, user_id: Snowflake) -> Option<ReadValue<'_, Snowflake, UserCache, SHB>> {
        self.map.get(&user_id).await
    }

    async fn get_cache_mut(&self, user_id: Snowflake) -> WriteValue<'_, Snowflake, UserCache, SHB> {
        self.map
            .get_mut_or_insert(&user_id, || UserCache {
                room: HashMap::with_hasher(self.map.hash_builder().clone()),
                rc: AtomicIsize::new(1), // initialize with one reference
            })
            .await
    }

    pub async fn set(&self, user_id: Snowflake, room_id: Snowflake, perm: PermMute) {
        self.get_cache_mut(user_id).await.room.insert(room_id, perm);
    }

    pub async fn batch_set(&self, user_id: Snowflake, iter: impl IntoIterator<Item = (Snowflake, PermMute)>) {
        self.get_cache_mut(user_id).await.room.extend(iter);
    }

    /// Increments the reference count if exists,
    /// returns true if and only if the cache was not stale or empty.
    ///
    /// NOTE: May return false AND increment the reference count, so `remove_reference`
    /// must always be called after this.
    pub async fn add_reference(&self, user_id: Snowflake) -> bool {
        match self.get_cache(user_id).await {
            None => false,
            Some(cache) => {
                // only return true when there was an existing reference,
                // don't allow stale results
                0 < cache.rc.fetch_add(1, Ordering::AcqRel)
            }
        }
    }

    pub async fn remove_reference(&self, user_id: Snowflake) {
        if let Some(cache) = self.get_cache(user_id).await {
            cache.rc.fetch_sub(1, Ordering::AcqRel);
        }
    }

    /// Cleanup any cache entries with no active users
    pub async fn cleanup(&self) {
        self.map
            .retain(|_, cache| cache.rc.load(Ordering::Acquire) > 0)
            .await
    }
}
