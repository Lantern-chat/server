use std::{
    hash::BuildHasher,
    sync::{
        atomic::{AtomicIsize, AtomicU64, Ordering},
        Arc,
    },
};

use hashbrown::HashMap;

/// Because the permission cache is a nested hashmap, we may as well share hashers.
#[derive(Default, Clone)]
struct SharedBuildHasher<S: BuildHasher>(Arc<S>);

impl<S: BuildHasher> BuildHasher for SharedBuildHasher<S> {
    type Hasher = <S as BuildHasher>::Hasher;

    fn build_hasher(&self) -> Self::Hasher {
        self.0.build_hasher()
    }
}

use schema::flags::RoomMemberFlags;
use sdk::models::{Permissions, Snowflake};

type UserId = Snowflake;
type RoomId = Snowflake;

type SHB = SharedBuildHasher<ahash::RandomState>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermMute {
    pub perms: Permissions,
    pub flags: RoomMemberFlags,
}

impl std::ops::Deref for PermMute {
    type Target = Permissions;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.perms
    }
}

// TODO: Maybe add per-party caching as well?
struct UserCache {
    room: HashMap<RoomId, PermMute, SHB>,
    rc: AtomicIsize,
}

pub struct PermissionCache {
    map: scc::HashMap<UserId, UserCache, SHB>,
    hb: SHB,
}

impl Default for PermissionCache {
    fn default() -> Self {
        let hb = SharedBuildHasher(Arc::new(ahash::RandomState::default()));

        PermissionCache {
            map: scc::HashMap::with_hasher(hb.clone()),
            hb,
        }
    }
}

impl PermissionCache {
    pub async fn has_user(&self, user_id: Snowflake) -> bool {
        self.map.contains_async(&user_id).await
    }

    pub async fn get(&self, user_id: Snowflake, room_id: Snowflake) -> Option<PermMute> {
        self.map
            .read_async(&user_id, |_, cache| {
                // double-check if not stale
                if cache.rc.load(Ordering::Acquire) > 0 {
                    cache.room.get(&room_id).copied()
                } else {
                    None
                }
            })
            .await
            .flatten()
    }

    async fn with_cache_mut<U>(&self, user_id: Snowflake, f: impl FnOnce(&mut UserCache) -> U) -> U {
        let mut entry = self.map.entry_async(user_id).await.or_insert_with(|| UserCache {
            room: HashMap::with_hasher(self.hb.clone()),
            rc: AtomicIsize::new(1), // initialize with one reference
        });

        f(entry.get_mut())
    }

    pub async fn set(&self, user_id: Snowflake, room_id: Snowflake, perm: PermMute) {
        self.with_cache_mut(user_id, |cache| {
            cache.room.insert(room_id, perm);
        })
        .await;
    }

    pub async fn batch_set(&self, user_id: Snowflake, iter: impl IntoIterator<Item = (Snowflake, PermMute)>) {
        self.with_cache_mut(user_id, |cache| {
            cache.room.extend(iter);
        })
        .await;
    }

    /// Increments the reference count if exists,
    /// returns true if and only if the cache was not stale or empty.
    ///
    /// NOTE: May return false AND increment the reference count, so `remove_reference`
    /// must always be called after this.
    pub async fn add_reference(&self, user_id: Snowflake) -> bool {
        // only return true when there was an existing reference, don't allow stale results
        Some(false) != self.map.read_async(&user_id, |_, cache| 0 < cache.rc.fetch_add(1, Ordering::AcqRel)).await
    }

    pub async fn remove_reference(&self, user_id: Snowflake) {
        self.map.read_async(&user_id, |_, cache| cache.rc.fetch_sub(1, Ordering::AcqRel)).await;
    }

    /// Cleanup any cache entries with no active users
    pub async fn cleanup(&self) {
        self.map.retain_async(|_, cache| cache.rc.load(Ordering::Acquire) > 0).await;
    }
}
