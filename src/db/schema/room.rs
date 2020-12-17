use crate::db::Snowflake;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum RoomKind {
    Private,
    Public,
    Direct,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: Snowflake,
    pub name: String,
    pub kind: RoomKind,
    pub party_id: Option<Snowflake>,
    pub avatar_id: Option<Snowflake>,
    pub topic: Option<String>,
    pub nsfw: bool,
}
