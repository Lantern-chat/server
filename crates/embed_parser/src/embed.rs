use smol_str::SmolStr;

use timestamp::Timestamp;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde_repr::Serialize_repr, serde_repr::Deserialize_repr)]
pub enum EmbedType {
    Image,
    Audio,
    Video,
    Html,
    Link,
}

#[derive(Clone)]
pub struct FullEmbed {
    pub ts: Timestamp,
    pub url: SmolStr,
    pub embed: Embed,
    pub expires: Option<Timestamp>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Embed {
    pub ty: EmbedType,

    #[serde(default, skip_serializing_if = "SmolStr::is_empty")]
    pub title: SmolStr,

    #[serde(default, skip_serializing_if = "SmolStr::is_empty")]
    pub description: SmolStr,

    #[serde(default, skip_serializing_if = "SmolStr::is_empty")]
    pub html: SmolStr,

    #[serde(default, skip_serializing_if = "SmolStr::is_empty")]
    pub provider_name: SmolStr,

    #[serde(default, skip_serializing_if = "SmolStr::is_empty")]
    pub provider_url: SmolStr,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,

    #[serde(default, skip_serializing_if = "SmolStr::is_empty")]
    pub thumbnail_url: SmolStr,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_width: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_height: Option<u32>,

    #[serde(default, skip_serializing_if = "SmolStr::is_empty")]
    pub alt: SmolStr,

    #[serde(default, skip_serializing_if = "SmolStr::is_empty")]
    pub mime: SmolStr,
}
