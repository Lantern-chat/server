use hashbrown::HashSet;
use sdk::Snowflake;

#[derive(Default)]
pub struct RoleCache {
    roles: HashSet<(Snowflake, Snowflake), ahash::RandomState>,
}

impl RoleCache {
    pub fn has(&self, party_id: Snowflake, role_id: Snowflake) -> bool {
        self.roles.contains(&(party_id, role_id))
    }

    pub fn remove_party(&mut self, party_id: Snowflake) {
        self.roles.retain(|&(pid, _)| pid != party_id);
    }

    pub fn remove_role(&mut self, party_id: Snowflake, role_id: Snowflake) {
        self.roles.remove(&(party_id, role_id));
    }

    pub fn add<'a>(&mut self, party_id: Snowflake, role_ids: impl IntoIterator<Item = &'a Snowflake>) {
        self.roles.extend(role_ids.into_iter().map(|&rid| (party_id, rid)));
    }
}
