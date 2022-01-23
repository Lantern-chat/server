use sdk::models::Snowflake;
use smol_str::SmolStr;

#[derive(Deserialize)]
pub struct RoomPatchOptions {
    pub id: Snowflake,

    #[serde(default)]
    pub name: Option<SmolStr>,
    #[serde(default)]
    pub topic: Option<SmolStr>,
    #[serde(default)]
    pub position: Option<i16>,
    #[serde(default)]
    pub parent_id: Option<Snowflake>,
}
