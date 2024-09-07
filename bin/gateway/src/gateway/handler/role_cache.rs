use hashbrown::HashSet;

use crate::prelude::*;

#[derive(Default)]
pub struct RoleCache {
    roles: HashSet<(PartyId, RoleId), sdk::FxRandomState2>,
}

impl RoleCache {
    pub fn has(&self, party_id: PartyId, role_id: RoleId) -> bool {
        self.roles.contains(&(party_id, role_id))
    }

    pub fn remove_party(&mut self, party_id: PartyId) {
        self.roles.retain(|&(pid, _)| pid != party_id);
    }

    pub fn remove_role(&mut self, party_id: PartyId, role_id: RoleId) {
        self.roles.remove(&(party_id, role_id));
    }

    pub fn add<'a>(&mut self, party_id: PartyId, role_ids: impl IntoIterator<Item = &'a RoleId>) {
        self.roles.extend(role_ids.into_iter().map(|&rid| (party_id, rid)));
    }
}
