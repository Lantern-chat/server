//use regex_automata::{Regex, RegexBuilder};
use smol_str::SmolStr;

use crate::{Error, ServerState};

//lazy_static::lazy_static! {
//    static ref VALID_URL: Regex = RegexBuilder::new()
//        .minimize(true)
//        .anchored(true)
//        .build("")
//        .unwrap();
//}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OEmbedFormat {
    Json,
    XML,
}

impl Default for OEmbedFormat {
    fn default() -> Self {
        OEmbedFormat::Json
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct OEmbedRequest {
    #[serde(default)]
    pub url: Option<SmolStr>,

    #[serde(default)]
    pub maxwidth: Option<u32>,

    #[serde(default)]
    pub maxheight: Option<u32>,

    #[serde(default)]
    pub format: OEmbedFormat,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum OEmebedType {
    Photo { url: SmolStr, width: u32, height: u32 },
    Video { html: SmolStr, width: u32, height: u32 },
    Link,
    Rich { html: SmolStr, width: u32, height: u32 },
}

#[derive(Debug, Clone, Serialize)]
pub struct OEmbedResponse {
    #[serde(flatten)]
    pub ty: OEmebedType,

    pub version: &'static str,
    pub provider_name: &'static str,
    pub provider_url: &'static str,
    pub cache_age: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_name: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_url: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_height: Option<u32>,
}

impl Default for OEmbedResponse {
    fn default() -> Self {
        OEmbedResponse {
            ty: OEmebedType::Link,
            version: "1.0",
            provider_name: "Lantern",
            provider_url: "https://lantern.chat",
            cache_age: 60 * 60 * 24,

            title: None,
            author_name: None,
            author_url: None,
            thumbnail_url: None,
            thumbnail_width: None,
            thumbnail_height: None,
        }
    }
}

pub async fn process_oembed(_state: &ServerState, _req: &OEmbedRequest) -> Result<OEmbedResponse, Error> {
    let url = SmolStr::new("https://lantern.chat/static/assets/preview.png");

    Ok(OEmbedResponse {
        ty: OEmebedType::Photo {
            url: url.clone(),
            height: 955,
            width: 1920,
        },
        title: Some(SmolStr::new_inline("Lantern Chat")),
        thumbnail_url: Some(url),
        thumbnail_height: Some(955),
        thumbnail_width: Some(1920),
        ..OEmbedResponse::default()
    })
}
