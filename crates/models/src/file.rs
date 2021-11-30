use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub id: Snowflake,
    pub filename: SmolStr,
    pub size: i64,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<SmolStr>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,

    /// Base-85 encoded blurhash, basically guaranteed to be larger than 22 bytes so just use a regular String
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}
