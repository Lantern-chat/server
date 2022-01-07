use super::*;

use hashbrown::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbedType {
    Image,
    Audio,
    Video,
    Html,
    Link,
    Article,
}

/// An embed is metadata taken from a given URL by loading said URL, parsing any meta tags, and fetching
/// extra information from oEmbed sources.
///
/// Typically, embeds contain title, description, etc. plus a thumbnail. However, direct media
/// may be embedded directly either via a URL (`embed_url`) or arbitrary HTML (`embed_html`), of which
/// should always be properly sandboxed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embed {
    /// Timestamp when the embed was retreived
    pub ts: Timestamp,

    /// URL fetched
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<SmolStr>,

    /// Embed type
    pub ty: EmbedType,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<SmolStr>,

    /// Description, usually from the Open-Graph API
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<SmolStr>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<i32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<EmbedAuthor>,

    /// oEmbed Provider
    #[serde(default, skip_serializing_if = "EmbedProvider::is_none")]
    pub provider: EmbedProvider,

    /// HTML and similar objects
    ///
    /// See: https://www.html5rocks.com/en/tutorials/security/sandboxed-iframes/
    #[serde(default, skip_serializing_if = "EmbedMedia::is_empty")]
    pub object: Option<EmbedMedia>,
    #[serde(default, skip_serializing_if = "EmbedMedia::is_empty")]
    pub image: Option<EmbedMedia>,
    #[serde(default, skip_serializing_if = "EmbedMedia::is_empty")]
    pub audio: Option<EmbedMedia>,
    #[serde(default, skip_serializing_if = "EmbedMedia::is_empty")]
    pub video: Option<EmbedMedia>,
    #[serde(default, skip_serializing_if = "EmbedMedia::is_empty")]
    pub thumb: Option<EmbedMedia>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<EmbedField>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct EmbedFooter {
    pub text: SmolStr,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<SmolStr>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_icon_url: Option<SmolStr>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct EmbedMedia {
    pub url: SmolStr,

    /// Non-visible description of the embedded media
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt: Option<SmolStr>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<SmolStr>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<SmolStr>,
}

impl EmbedMedia {
    pub fn is_empty(this: &Option<EmbedMedia>) -> bool {
        match this {
            Some(ref e) => e.url.is_empty(),
            None => true,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct EmbedProvider {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<SmolStr>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<SmolStr>,
}

impl EmbedProvider {
    pub const fn is_none(&self) -> bool {
        self.name.is_none() && self.url.is_none()
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct EmbedAuthor {
    pub name: SmolStr,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<SmolStr>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<SmolStr>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_icon_url: Option<SmolStr>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct EmbedField {
    name: SmolStr,
    value: SmolStr,

    #[serde(default, skip_serializing_if = "crate::is_false")]
    inline: bool,
}

impl EmbedField {
    pub fn is_empty(&self) -> bool {
        self.name.is_empty() || self.value.is_empty()
    }
}

impl Default for Embed {
    #[inline]
    fn default() -> Embed {
        Embed {
            ts: Timestamp::now_utc(),
            ty: EmbedType::Link,
            url: None,
            title: None,
            description: None,
            color: None,
            author: None,
            provider: EmbedProvider::default(),
            image: None,
            audio: None,
            video: None,
            thumb: None,
            object: None,
            fields: Vec::new(),
        }
    }
}
