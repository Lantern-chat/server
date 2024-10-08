use std::sync::atomic::{AtomicIsize, Ordering};

use hashbrown::HashMap;

use schema::flags::RoomMemberFlags;
use sdk::models::*;

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

struct UserCache {
    room: HashMap<RoomId, PermMute, sdk::FxRandomState2>,
    rc: AtomicIsize,
}

impl Default for UserCache {
    fn default() -> Self {
        Self {
            room: HashMap::default(),
            rc: AtomicIsize::new(1),
        }
    }
}

#[derive(Default)]
pub struct PermissionCache {
    map: scc::HashMap<UserId, UserCache, sdk::FxRandomState2>,
}

impl PermissionCache {
    pub async fn has_user(&self, user_id: UserId) -> bool {
        self.map.contains_async(&user_id).await
    }

    pub async fn get(&self, user_id: UserId, room_id: RoomId) -> Option<PermMute> {
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

    async fn with_cache_mut<U>(&self, user_id: UserId, f: impl FnOnce(&mut UserCache) -> U) -> U {
        let mut entry = self.map.entry_async(user_id).await.or_default();

        f(entry.get_mut())
    }

    pub async fn set(&self, user_id: UserId, room_id: RoomId, perm: PermMute) {
        self.with_cache_mut(user_id, |cache| {
            cache.room.insert(room_id, perm);
        })
        .await;
    }

    pub async fn batch_set(&self, user_id: UserId, iter: impl IntoIterator<Item = (Snowflake, PermMute)>) {
        self.with_cache_mut(user_id, |cache| {
            cache.room.extend(iter);
            if cache.room.capacity() > (cache.room.len() * 3 / 2) {
                cache.room.shrink_to_fit();
            }
        })
        .await;
    }

    pub async fn remove(&self, user_id: UserId, room_id: RoomId) -> bool {
        self.map.update_async(&user_id, |_, cache| cache.room.remove(&room_id)).await.flatten().is_some()
    }

    pub async fn clear_user(&self, user_id: UserId) -> bool {
        self.map.update_async(&user_id, |_, cache| cache.room.clear()).await.is_some()
    }

    /// Increments the reference count if exists,
    /// returns true if and only if the cache was not stale or empty.
    ///
    /// NOTE: May return false AND increment the reference count, so `remove_reference`
    /// must always be called after this.
    pub async fn add_reference(&self, user_id: UserId) -> bool {
        // only return true when there was an existing reference, don't allow stale results
        Some(false) != self.map.read_async(&user_id, |_, cache| 0 < cache.rc.fetch_add(1, Ordering::AcqRel)).await
    }

    #[rustfmt::skip]
    pub async fn remove_reference(&self, user_id: UserId) {
        self.map.update_async(&user_id, |_, cache| {
            if 1 == cache.rc.fetch_sub(1, Ordering::AcqRel) {
                cache.room.clear();
            }
        }).await;
    }

    /// Cleanup any cache entries with no active users
    pub async fn cleanup(&self) {
        self.map.retain_async(|_, cache| cache.rc.load(Ordering::Acquire) > 0).await;
    }
}
