use super::*;

bitflags::bitflags! {
    pub struct RoleFlags: i16 {
        const HOIST         = 1 << 0;
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

    // TODO: Revist removing this
    pub party_id: Snowflake,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<SmolStr>,
    pub name: Option<SmolStr>,
    pub permissions: Permission,
    pub color: Option<u32>,
    pub position: i16,
    pub flags: RoleFlags,
}

impl Role {
    pub fn is_mentionable(&self) -> bool {
        self.flags.contains(RoleFlags::MENTIONABLE)
    }

    pub fn is_admin(&self) -> bool {
        self.permissions.is_admin()
    }
}
