use crate::db::Snowflake;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Emote {
    pub id: Snowflake,
    pub party_id: Snowflake,
    pub name: String,
    pub animated: bool,
    pub aspect_ratio: f32,
    pub sticker: bool,
    pub data: Vec<u8>,
}
