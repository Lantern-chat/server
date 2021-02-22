use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: Snowflake,
    pub party_id: Snowflake,
    pub name: String,
    pub admin: bool,
    pub permissions: u32, // replace with bitflags!
    pub color: u32,
    pub mentionable: bool,
}

impl Role {}
