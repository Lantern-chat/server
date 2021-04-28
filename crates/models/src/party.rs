use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    pub id: Snowflake,
    pub owner: Snowflake,
    pub name: String,

    pub roles: Vec<Role>,
    pub emotes: Vec<Emote>,
}
