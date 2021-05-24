use super::*;

bitflags::bitflags! {
    pub struct RoleFlags: i16 {
        const ADMIN         = 1 << 0;
        const MENTIONABLE   = 1 << 1;
    }
}

serde_shims::impl_serde_for_bitflags!(RoleFlags);

impl Default for RoleFlags {
    fn default() -> Self {
        RoleFlags::empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: Snowflake,
    pub party_id: Snowflake,
    pub name: Option<String>,
    pub permissions: Permission,
    pub color: Option<u32>,
    pub flags: RoleFlags,
}

impl Role {
    pub fn is_mentionable(&self) -> bool {
        self.flags.contains(RoleFlags::MENTIONABLE)
    }

    pub fn is_admin(&self) -> bool {
        self.flags.contains(RoleFlags::ADMIN)
    }
}
