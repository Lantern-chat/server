use futures::{FutureExt, StreamExt};
use sdk::models::gateway::events::Ready;
use sdk::models::{aliases::*, Overwrite, Permissions, Room};

use parking_lot::RwLock;
use thin_vec::ThinVec;
use tokio::sync::RwLock as AsyncRwLock;
use triomphe::Arc;

use rkyv::Archived;

#[derive(Default)]
pub struct StructureCache {
    pub role_perms: scc::HashIndex<RoleId, Permissions, ahash::RandomState>,
    pub user_roles: scc::HashIndex<(PartyId, UserId), Arc<RwLock<VecSet<RoleId>>>, ahash::RandomState>,
    pub rooms: scc::HashIndex<RoomId, RoomStructure, ahash::RandomState>,
    pub parties: scc::HashIndex<PartyId, Arc<PartyStructure>, ahash::RandomState>,
}

pub struct PartyStructure {
    pub owner_id: UserId,
    pub rooms: AsyncRwLock<VecSet<RoomId>>,
    pub roles: AsyncRwLock<VecSet<RoleId>>,
}

#[derive(Clone)]
pub struct RoomStructure {
    pub party_id: PartyId,
    pub overwrites: Arc<[Overwrite]>,
}

impl RoomStructure {
    pub fn is_same(&self, room: &Archived<Room>) -> bool {
        self.party_id == room.party_id && &*self.overwrites == &*room.overwrites
    }
}

impl StructureCache {
    pub async fn populate_from_ready(&self, ready: &Archived<Ready>) {
        // each ready event contains a list of all rooms the user is in, and all the parties.
        // each room includes all overwrites for that room, and each party includes all roles.

        use scc::hash_index::Entry;

        let update_rooms = async {
            let _guard = scc::ebr::Guard::new();

            for new_room in ready.rooms.iter() {
                if let Some(room) = self.rooms.peek(&new_room.id, &_guard) {
                    if room.is_same(new_room) {
                        continue;
                    }
                }

                let room = RoomStructure {
                    party_id: new_room.party_id,
                    // slice::Iter.map should be exact-size and thus only incur one allocation
                    overwrites: Arc::from_iter(new_room.overwrites.iter().map(|overwrite| Overwrite {
                        id: overwrite.id,
                        allow: overwrite.allow,
                        deny: overwrite.deny,
                    })),
                };

                match self.rooms.entry_async(new_room.id).await {
                    Entry::Occupied(entry) => {
                        if !entry.get().is_same(new_room) {
                            entry.update(room);
                        }
                    }
                    Entry::Vacant(entry) => _ = entry.insert_entry(room),
                }
            }
        };

        let update_parties = async {
            let _guard = scc::ebr::Guard::new();

            for new_party in ready.parties.iter() {}
        };

        todo!("populate_from_ready")
    }

    /// Remove a party from the cache, removing all rooms and role permissions associated with it.
    pub async fn remove_party(&self, party_id: PartyId) {
        let mut party = None;

        // remove from table and get the party structure at the same time
        let removed = self.parties.remove_if_async(&party_id, |p| {
            party = Some(p.clone());
            true
        });

        let (true, Some(party)) = (removed.await, party) else { return };

        tokio::join!(
            async {
                for room_id in party.rooms.read().await.iter() {
                    self.rooms.remove_async(room_id).await;
                }

                party.rooms.write().await.clear();
            },
            async {
                for role_id in party.roles.read().await.iter() {
                    self.role_perms.remove_async(role_id).await;
                }

                party.roles.write().await.clear();
            }
        );
    }

    /// Compute the [`Permissions`] for a user in a room.
    ///
    /// If this returns `None`, the data is not cached.
    ///
    /// If the user is not a member of the party, this will return an empty [`Permissions`] set.
    ///
    /// This method is accurate to the current cache state but is slow to compute.
    /// An extra local cache should be used on top of this method to avoid recomputing permissions.
    pub async fn compute_permissions_slow(&self, room_id: RoomId, user_id: UserId) -> Option<Permissions> {
        let room = self.rooms.get_async(&room_id).await?.get().clone();

        // combined key for user roles
        let user_roles_key = (room.party_id, user_id);

        // fetch roles and party concurrently
        let (roles, party) = tokio::join! {
            self.user_roles.get_async(&user_roles_key),
            self.parties.get_async(&room.party_id),
        };

        let roles = match roles {
            Some(roles) => roles.get().clone(),

            // if this is not found, the user is not a member of the party
            None => return Some(Permissions::empty()),
        };

        // owners implicitly have all permissions
        if party?.get().owner_id == user_id {
            return Some(Permissions::all());
        }

        // RwLock here should almost always succeed immediately
        let roles = roles.read();

        let mut base_perms = Permissions::empty();

        // iterate over all roles concurrently and merge their permissions
        futures::stream::iter([&room.party_id]) // include @everyone role
            .chain(futures::stream::iter(roles.iter()))
            .for_each_concurrent(16, |role| async move {
                let Some(perms) = self.role_perms.get_async(&role).await else { return };

                base_perms |= perms.get().clone();
            })
            .await;

        if base_perms.is_empty() {
            return None;
        }

        Some(base_perms.compute_overwrites(&room.overwrites, &roles, user_id))
    }

    /// Compute the [`Permissions`] for a user in a room.
    ///
    /// If this returns `None`, the data is not cached.
    ///
    /// If the user is not a member of the party, this will return an empty [`Permissions`] set.
    ///
    /// This method is well-optimized but not instant, and an extra local cache on top should be used.
    ///
    /// This method is also not linearizable, and may return stale data if the cache is being actively updated.
    pub fn compute_permissions_fast(&self, room_id: RoomId, user_id: UserId) -> Option<Permissions> {
        let _guard = scc::ebr::Guard::new();

        let room = self.rooms.peek(&room_id, &_guard)?.clone();

        let roles = match self.user_roles.peek(&(room.party_id, user_id), &_guard) {
            Some(roles) => roles.clone(),

            // if this is not found, the user is not a member of the party
            None => return Some(Permissions::empty()),
        };

        // owners implicitly have all permissions
        if user_id == self.parties.peek(&room.party_id, &_guard)?.owner_id {
            return Some(Permissions::all());
        }

        // get @everyone perms
        let mut base_perms = self.role_perms.peek(&room.party_id, &_guard)?.clone();

        // RwLock here should almost always succeed immediately
        let roles = roles.read();

        for role_id in roles.iter() {
            let Some(role_perms) = self.role_perms.peek(role_id, &_guard).cloned() else {
                log::warn!("Missing cached role permissions for {}", role_id);
                continue;
            };

            base_perms |= role_perms;
        }

        drop(_guard);

        Some(base_perms.compute_overwrites(&room.overwrites, &roles, user_id))
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct VecSet<T>(ThinVec<T>);

impl<T> std::ops::Deref for VecSet<T> {
    type Target = ThinVec<T>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for VecSet<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: PartialEq> VecSet<T> {
    pub fn insert(&mut self, item: T) {
        if !self.contains(&item) {
            self.push(item);
        }
    }

    pub fn remove(&mut self, item: &T) {
        self.retain(|i| i != item);
    }

    pub fn from_iter_ordered<I: IntoIterator<Item = T>>(iter: I) -> Self
    where
        T: Ord,
    {
        let mut vec = ThinVec::from_iter(iter);
        vec.sort_unstable();
        vec.dedup();
        Self(vec)
    }
}
