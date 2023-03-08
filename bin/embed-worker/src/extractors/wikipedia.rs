use hashbrown::HashMap;

use super::prelude::*;

#[derive(Debug)]
pub struct WikipediaExtractorFactory;

#[derive(Debug)]
pub struct WikipediaExtractor {
    pub max_sentences: u8,
    pub thumbnail_size: u16,
}

impl ExtractorFactory for WikipediaExtractorFactory {
    fn create(&self, config: &Config) -> Result<Option<Box<dyn Extractor>>, ConfigError> {
        let mut wiki = Box::new(WikipediaExtractor {
            max_sentences: 4,
            thumbnail_size: 256,
        });

        if let Some(extractor) = config.parsed.extractors.get("wikipedia") {
            if let Some(max_sentences) = extractor.get("max_sentences") {
                match max_sentences.parse() {
                    Ok(max_sentences) => wiki.max_sentences = max_sentences,
                    Err(_) => return Err(ConfigError::InvalidExtractorField("wikipedia.max_sentences")),
                }
            }

            if let Some(thumbnail_size) = extractor.get("thumbnail_size") {
                match thumbnail_size.parse() {
                    Ok(thumbnail_size) => wiki.thumbnail_size = thumbnail_size,
                    Err(_) => return Err(ConfigError::InvalidExtractorField("wikipedia.thumbnail_size")),
                }
            }
        };

        Ok(Some(wiki))
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
            async { state.client.get(text_extract_uri).send().await?.json::<WikipediaTextResult>().await },
            async { state.client.get(thumbnail_extract_uri).send().await?.json::<WikipediaImageResult>().await },
        }?;

        let mut embed = EmbedV1::default();

        if let Some(TextPage::Found { title, extract }) = text.query.pages.get(0) {
            embed.title = Some(title.clone());
            embed.description = Some(extract.into());
        } else {
            return Err(Error::Failure(StatusCode::NOT_FOUND));
        }

        if let Some(ImagePage::Found { thumbnail, pageimage }) = image.query.pages.values().next() {
            let mut media = BoxedEmbedMedia::default().with_url(&thumbnail.source);

            media.width = thumbnail.width;
            media.height = thumbnail.height;
            media.description = Some(pageimage.into());
            embed.thumb = Some(media);
        }

        embed.url = Some(smol_str::format_smolstr!("{origin}/wiki/{title}"));
        embed.color = Some(0xFFFFFF); // white

        embed.provider.name = Some(SmolStr::new_inline("Wikipedia"));
        embed.provider.icon = Some(
            BoxedEmbedMedia::default()
                .with_url(smol_str::format_smolstr!("{origin}/static/favicon/wikipedia.ico")),
        );

        // 4-hour expire
        Ok(generic::finalize_embed(state, embed, Some(60 * 60 * 4)))
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
