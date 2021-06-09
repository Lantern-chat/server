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

use db::Snowflake;

use models::Permission;

type UserId = Snowflake;
type RoomId = Snowflake;

type SHB = SharedBuildHasher<DefaultHashBuilder>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermMute {
    pub perm: Permission,
    pub muted: bool,
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
                SharedBuildHasher(Arc::new(DefaultHashBuilder::new())),
            ),
        }
    }

    pub async fn has_user(&self, user_id: Snowflake) -> bool {
        self.map.contains(&user_id).await
    }

    pub async fn get(&self, user_id: Snowflake, room_id: Snowflake) -> Option<PermMute> {
        self.map
            .get(&user_id)
            .await
            .and_then(|rooms| rooms.room.get(&room_id).copied())
    }

    #[inline]
    async fn get_cache(&self, user_id: Snowflake) -> Option<ReadValue<'_, Snowflake, UserCache, SHB>> {
        self.map.get(&user_id).await
    }

    async fn get_cache_mut(&self, user_id: Snowflake) -> WriteValue<'_, Snowflake, UserCache, SHB> {
        self.map
            .get_mut_or_insert(&user_id, || UserCache {
                room: HashMap::with_hasher(self.map.hash_builder().clone()),
                rc: AtomicIsize::new(1),
            })
            .await
    }

    pub async fn set(&self, user_id: Snowflake, room_id: Snowflake, perm: PermMute) {
        self.get_cache_mut(user_id).await.room.insert(room_id, perm);
    }

    pub async fn batch_set(&self, user_id: Snowflake, iter: impl IntoIterator<Item = (Snowflake, PermMute)>) {
        self.get_cache_mut(user_id).await.room.extend(iter);
    }

    pub async fn add_reference(&self, user_id: Snowflake) -> bool {
        if let Some(cache) = self.get_cache(user_id).await {
            cache.rc.fetch_add(1, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    pub async fn remove_reference(&self, user_id: Snowflake) {
        if let Some(cache) = self.get_cache(user_id).await {
            cache.rc.fetch_sub(1, Ordering::SeqCst);
        }
    }

    /// Cleanup any cache entries with no active users
    pub async fn cleanup(&self) {
        self.map
            .retain(|_, cache| cache.rc.load(Ordering::SeqCst) > 0)
            .await
    }
}
