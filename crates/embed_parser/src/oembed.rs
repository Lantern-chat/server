//use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OEmbedFormat {
    JSON = 1,
    XML = 2,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OEmbedLink<'a> {
    pub url: &'a str,
    pub title: Option<&'a str>,
    pub format: OEmbedFormat,
}

pub type LinkList<'a> = smallvec::SmallVec<[OEmbedLink<'a>; 1]>;

pub fn parse_link_header<'a>(header: &'a str) -> LinkList<'a> {
    let mut res = LinkList::default();

    // multiple links can be comma-separated
    'links: for link in header.split(',') {
        let mut parts = link.split(';').map(str::trim);

        let url = match parts.next() {
            Some(url) if url.starts_with("<http") && url.ends_with('>') => &url[1..url.len() - 1],
            _ => continue,
        };

        let mut link = OEmbedLink {
            url,
            title: None,
            format: OEmbedFormat::JSON,
        };

        //while let Some(part) = parts.next() {
        for part in parts {
            let Some((left, right)) = part.split_once('=') else {
                continue 'links
            };

            if left == "type" && right.contains("xml") {
                link.format = OEmbedFormat::XML;
                continue;
            }

            let right = crate::trim_quotes(right);

            match left {
                "title" => link.title = Some(right),
                "rel" if right != "alternate" => continue 'links,
                _ => continue,
            }
        }

        res.push(link);
    }

    res.sort_by_key(|r| r.format);

    res
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OEmbedType {
    Photo,
    Video,
    Link,
    Rich,

    #[serde(other)]
    Unknown,
}

use smol_str::SmolStr;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OEmbed {
    pub version: monostate::MustBe!("1.0"),

    #[serde(rename = "type")]
    pub kind: OEmbedType,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<SmolStr>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_name: Option<SmolStr>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_url: Option<SmolStr>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_name: Option<SmolStr>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_url: Option<SmolStr>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_age: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<SmolStr>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_width: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_height: Option<i32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<SmolStr>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html: Option<SmolStr>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
}

impl OEmbed {
    pub const fn is_valid(&self) -> bool {
        let has_dimensions = self.width.is_some() && self.height.is_some();

        match self.kind {
            OEmbedType::Video | OEmbedType::Rich => self.html.is_some() && has_dimensions,
            OEmbedType::Photo => self.url.is_some() && has_dimensions,
            _ => true,
        }
    }
}
