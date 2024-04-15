use sdk::{
    models::{Overwrite, Permissions},
    Snowflake,
};

use parking_lot::RwLock;
use thin_vec::ThinVec;
use tokio::sync::RwLock as AsyncRwLock;

use triomphe::Arc;

pub type UserId = Snowflake;
pub type RoleId = Snowflake;
pub type RoomId = Snowflake;
pub type PartyId = Snowflake;

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

use rkyv::Archived;

use sdk::models::gateway::events::Ready;

impl StructureCache {
    pub async fn populate_from_ready(&self, ready: &Archived<Ready>) {
        todo!("populate_from_ready")
    }

    pub fn compute_overwrites(&self, room_id: RoomId, user_id: UserId) -> Option<Permissions> {
        let _guard = scc::ebr::Guard::new();

        let room = self.rooms.peek(&room_id, &_guard)?.clone();
        let party_id = room.party_id;

        // owners implicitly have all permissions
        if user_id == self.parties.peek(&party_id, &_guard)?.owner_id {
            return Some(Permissions::all());
        }

        let user_roles = self.user_roles.peek(&(party_id, user_id), &_guard).cloned();

        // get @everyone perms
        let mut base_perms = self.role_perms.peek(&party_id, &_guard)?.clone();

        Some(if let Some(ref roles) = user_roles {
            // should almost always succeed immediately
            let roles = roles.read();

            for role_id in roles.iter() {
                let Some(role_perms) = self.role_perms.peek(role_id, &_guard).cloned() else {
                    log::warn!("Missing cached role permissions for {}", role_id);
                    continue;
                };

                base_perms |= role_perms;
            }

            drop(_guard);

            base_perms.compute_overwrites(&room.overwrites, &roles, user_id)
        } else {
            drop(_guard);

            base_perms.compute_overwrites(&room.overwrites, &[], user_id)
        })
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
}
