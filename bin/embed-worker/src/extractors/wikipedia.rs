use std::fmt::Write;
use std::sync::Arc;

use crate::{Error, Params, Site, WorkerState};
use hashbrown::HashMap;
use sdk::models::embed::v1::*;
use sdk::models::SmolStr;
use smol_str::ToSmolStr;
use url::Url;

use embed_parser::oembed::{OEmbed, OEmbedFormat, OEmbedLink};
use futures_util::FutureExt;
use reqwest::{header::HeaderName, Method, StatusCode};

use super::{Config, ConfigError, EmbedWithExpire, Extractor, ExtractorFactory};

#[derive(Debug)]
pub struct WikipediaExtractorFactory;

#[derive(Debug)]
pub struct WikipediaExtractor {
    pub max_sentences: u8,
    pub thumbnail_size: u16,
}

impl ExtractorFactory for WikipediaExtractorFactory {
    fn create(&self, _config: &Config) -> Result<Option<Box<dyn Extractor>>, ConfigError> {
        Ok(Some(Box::new(WikipediaExtractor {
            max_sentences: 4,
            thumbnail_size: 256,
        })))
    }
}

#[async_trait::async_trait]
impl Extractor for WikipediaExtractor {
    fn matches(&self, url: &Url) -> bool {
        matches!(url.domain(), Some(domain) if domain.ends_with("wikipedia.org"))
            && url.path().starts_with("/wiki/")
    }

    async fn extract(&self, state: Arc<WorkerState>, url: Url, params: Params) -> Result<EmbedWithExpire, Error> {
        let Some(title) = url.path_segments().and_then(|mut s| s.nth(1)) else {
            return Err(Error::Failure(StatusCode::NOT_FOUND));
        };

        let origin = url.origin().ascii_serialization();

        let text_extract_uri = format!(
            "{origin}/w/api.php?action=query&prop=extracts&exsentences={}&exlimit=1&titles={title}&explaintext=1&formatversion=2&format=json",
            self.max_sentences
        );

        let thumbnail_extract_uri = format!(
            "{origin}/w/api.php?action=query&titles={title}&prop=pageimages&format=json&pithumbsize={}",
            self.thumbnail_size
        );

        let (text, image) = tokio::try_join! {
            async {
                std::fs::write("./text.json", state.client.get(text_extract_uri.clone()).send().await?.text().await?).unwrap();
                state.client.get(text_extract_uri).send().await?.json::<WikipediaTextResult>().await
            },
            async {
                std::fs::write("./image.json", state.client.get(thumbnail_extract_uri.clone()).send().await?.text().await?).unwrap();
                state.client.get(thumbnail_extract_uri).send().await?.json::<WikipediaImageResult>().await
            },
        }?;

        let mut embed = EmbedV1::default();

        if let Some(TextPage::Found { title, extract }) = text.query.pages.get(0) {
            embed.title = Some(title.clone());
            embed.description = Some(extract.into());
        } else {
            return Err(Error::Failure(StatusCode::NOT_FOUND));
        }

        if let Some(ImagePage::Found { thumbnail, pageimage }) = image.query.pages.values().next() {
            let mut media = Box::<EmbedMedia>::default();
            media.url = (&thumbnail.source).into();
            media.width = thumbnail.width;
            media.height = thumbnail.height;
            media.description = Some(pageimage.into());
            embed.thumb = Some(media);
        }

        embed.url = Some(smol_str::format_smolstr!("{origin}/wiki/{title}"));
        embed.color = Some(0xFFFFFF); // white
        embed.provider.name = Some(SmolStr::new_inline("Wikipedia"));
        embed.provider.icon = Some({
            let mut media = Box::<EmbedMedia>::default();
            media.url = smol_str::format_smolstr!("{origin}/static/favicon/wikipedia.ico");
            media
        });

        embed_parser::quirks::fix_embed(&mut embed);

        embed.visit_media_mut(|media| {
            media.signature = state.sign(&media.url);
        });

        Ok(super::generic::compute_expirey(embed, 60 * 60))
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct WikipediaTextResult {
    pub query: WikipediaTextQuery,
}

#[derive(Debug, serde::Deserialize)]
pub struct WikipediaImageResult {
    pub query: WikipediaImageQuery,
}

#[derive(Debug, serde::Deserialize)]
pub struct WikipediaTextQuery {
    pub pages: Vec<TextPage>,
}

#[derive(Debug, serde::Deserialize)]
pub struct WikipediaImageQuery {
    pub pages: HashMap<SmolStr, ImagePage>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum TextPage {
    Found { title: SmolStr, extract: String },
    NotFound {},
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum ImagePage {
    Found { thumbnail: Thumbnail, pageimage: String },
    NotFound {},
}

#[derive(Debug, serde::Deserialize)]
pub struct Thumbnail {
    pub source: String,

    #[serde(default)]
    pub width: Option<i32>,
    #[serde(default)]
    pub height: Option<i32>,
}
