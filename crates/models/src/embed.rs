use super::*;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde_repr::Serialize_repr, serde_repr::Deserialize_repr)]
pub enum EmbedType {
    Image,
    Audio,
    Video,
    Html,
    Link,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Embed {
    /// Timestamp when the embed was retreived
    pub ts: Timestamp,

    /// URL fetched
    pub url: SmolStr,

    /// Embed type
    pub ty: EmbedType,

    /// Title, usually from the Open-Graph API
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<SmolStr>,

    /// Description, usually from the Open-Graph API
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<SmolStr>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<i32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<SmolStr>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_url: Option<SmolStr>,

    /// HTML Markup to embed in iframe
    ///
    /// See: https://www.html5rocks.com/en/tutorials/security/sandboxed-iframes/
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html: Option<SmolStr>,

    /// oEmbed Provider Name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_name: Option<SmolStr>,

    /// oEmbeed Provider URL
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_url: Option<SmolStr>,

    /// Height of embedded object
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,

    /// Width of embedded object
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<SmolStr>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_width: Option<i32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_height: Option<i32>,

    /// Non-visible description of the embedded media
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt: Option<SmolStr>,

    /// Mime type of embed preview
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<SmolStr>,
}

impl Embed {
    #[inline]
    pub fn new(url: SmolStr) -> Embed {
        Embed {
            ts: Timestamp::now_utc(),
            ty: EmbedType::Link,
            url,
            title: None,
            description: None,
            color: None,
            author: None,
            author_url: None,
            html: None,
            provider_name: None,
            provider_url: None,
            height: None,
            width: None,
            thumbnail_url: None,
            thumbnail_width: None,
            thumbnail_height: None,
            alt: None,
            mime: None,
        }
    }
}
