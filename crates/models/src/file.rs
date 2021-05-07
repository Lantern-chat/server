use crate::Snowflake;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub id: Snowflake,
    pub filename: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
}
