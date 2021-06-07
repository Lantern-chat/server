use std::{hash::BuildHasher, sync::Arc};

use hashbrown::HashMap;

use util::cmap::{CHashMap, DefaultHashBuilder};

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

pub struct PermissionCache {
    map: CHashMap<UserId, HashMap<RoomId, Permission, SHB>, SHB>,
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

    pub async fn get(&self, user_id: Snowflake, room_id: Snowflake) -> Option<Permission> {
        self.map
            .get(&user_id)
            .await
            .and_then(|rooms| rooms.get(&room_id).copied())
    }

    pub async fn set(&self, user_id: Snowflake, room_id: Snowflake, perm: Permission) {
        self.map
            .get_mut_or_insert(&user_id, || {
                HashMap::with_hasher(self.map.hash_builder().clone())
            })
            .await
            .insert(room_id, perm);
    }

    pub async fn batch_set(
        &self,
        user_id: Snowflake,
        iter: impl IntoIterator<Item = (Snowflake, Permission)>,
    ) {
        self.map
            .get_mut_or_insert(&user_id, || {
                HashMap::with_hasher(self.map.hash_builder().clone())
            })
            .await
            .extend(iter);
    }

    pub async fn clear(&self, user_id: Snowflake) {
        self.map.remove(&user_id).await;
    }
}
