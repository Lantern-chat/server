use smol_str::SmolStr;

#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbedType {
    Image,
    Audio,
    Video,
    Html,
    Link,
}

#[derive(Clone)]
pub struct AggregateEmbed {
    pub title: SmolStr,
    pub description: SmolStr,
    pub ty: EmbedType,
    pub url: SmolStr,
    pub html: SmolStr,
    pub provider_name: SmolStr,
    pub provider_url: SmolStr,
    pub height: Option<u32>,
    pub width: Option<u32>,
    pub thumbnail_url: SmolStr,
    pub thumbnail_width: Option<u32>,
    pub thumbnail_height: Option<u32>,
    pub alt: SmolStr,
    pub mime: SmolStr,
}
