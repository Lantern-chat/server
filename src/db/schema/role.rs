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

impl Role {
    pub fn from_row(row: &Row) -> Self {
        Role {
            id: row.get(0),
            party_id: row.get(1),
            name: row.get(2),
            admin: row.get(3),
            permissions: row.get(4),
            color: row.get(5),
            mentionable: row.get(6),
        }
    }
}
