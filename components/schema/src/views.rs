use super::*;

use smol_str::SmolStr;

#[derive(Debug, serde::Deserialize)]
pub struct AggAttachmentsMeta {
    pub id: Snowflake,
    pub size: i32,
    pub name: SmolStr,

    #[serde(default)]
    pub mime: Option<SmolStr>,

    #[serde(default)]
    pub width: Option<i32>,

    #[serde(default)]
    pub height: Option<i32>,

    #[serde(default)]
    pub flags: Option<i16>,
}
