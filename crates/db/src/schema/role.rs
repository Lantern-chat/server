use super::*;

use models::permission::Permission;

#[derive(Debug, Clone)]
pub struct Role {
    pub id: Snowflake,
    pub party_id: Snowflake,
    pub name: String,
    pub admin: bool,
    pub permissions: Permission,
    pub color: u32,
    pub mentionable: bool,
}

impl Role {}
