use super::prelude::*;

pub struct ImgurExtractorFactory;

#[derive(Debug)]
pub struct ImgurExtractor {
    pub client_id: HeaderValue,
}

impl ExtractorFactory for ImgurExtractorFactory {
    fn create(&self, config: &Config) -> Result<Option<Box<dyn Extractor>>, ConfigError> {
        let Some(extractor) = config.parsed.extractors.get("imgur") else {
            return Ok(None);
        };

        let Some(client_id) = extractor.get("client_id") else {
            return Err(ConfigError::MissingExtractorField("imgur.client_id"));
        };

        let Ok(client_id) = HeaderValue::try_from(format!("Client-ID {client_id}")) else {
            return Err(ConfigError::InvalidExtractorField("imgur.client_id"));
        };

        Ok(Some(Box::new(ImgurExtractor { client_id })))
    }
}

// These are just some known path segments that can't be embedded
const BAD_PATHS: &[&str] = &[
    "user", "upload", "signin", "emerald", "vidgif", "memegen", "apps", "search",
];

#[async_trait::async_trait]
impl Extractor for ImgurExtractor {
    fn matches(&self, url: &Url) -> bool {
        if !matches!(url.domain(), Some("imgur.com" | "i.imgur.com")) {
            return false;
        }

        let Some(mut segments) = url.path_segments() else {
            return false;
        };

        let potential_image_id = match segments.next() {
            Some("gallery" | "a") => match segments.next() {
                Some(potential_image_id) => potential_image_id,
                None => return false,
            },
            Some(potential_image_id) if !BAD_PATHS.contains(&potential_image_id) => potential_image_id,
            _ => return false,
        };

        // strip file extension if present
        let Some(potential_image_id) = potential_image_id.split('.').next() else {
            return false;
        };

        potential_image_id.chars().all(|c| c.is_ascii_alphanumeric())
    }

    async fn extract(&self, state: Arc<WorkerState>, url: Url, params: Params) -> Result<EmbedWithExpire, Error> {
        let Some(mut segments) = url.path_segments() else {
            return Err(Error::Failure(StatusCode::NOT_FOUND));
        };

        let (id, api) = match segments.next() {
            Some(seg @ ("gallery" | "a")) => match segments.next() {
                Some(id) => (id, if seg == "a" { "album" } else { "gallery/album" }),
                None => unreachable!(),
            },
            Some(id) => (id, "image"),
            _ => unreachable!(),
        };

        // strip file extension if present
        let Some(id) = id.split('.').next() else {
            return Err(Error::Failure(StatusCode::NOT_FOUND));
        };

        let resp = state
            .client
            .get(format!("https://api.imgur.com/3/{api}/{id}"))
            .header(HeaderName::from_static("authorization"), &self.client_id)
            .send()
            .await?
            .json()
            .await?;

        let ImgurResult::Success { data: Some(mut data), .. } = resp else {
            return Err(Error::Failure(StatusCode::NOT_FOUND));
        };

        let mut embed = EmbedV1::default();

        #[rustfmt::skip]
        let image = match &mut data.kind {
            | ImgurDataKind::Gallery { images, .. }
            | ImgurDataKind::Album { images, .. } => match data.cover {
                Some(ref cover) => match images.iter_mut().find(|img| img.id == *cover) {
                    Some(image) => Some(image),
                    None => images.get_mut(0),
                },
                None => images.get_mut(0)
            },
            ImgurDataKind::Image { image } => Some(image),
        };

        let Some(image) = image else {
            return Err(Error::Failure(StatusCode::NOT_FOUND));
        };

        let mut media = BoxedEmbedMedia::default();

        // add ?noredirect to imgur links because they're annoying
        media.url = add_noredirect(std::mem::take(&mut image.link)).into();

        media.width = image.width;
        media.height = image.height;

        match image.mime.take() {
            Some(mime) if mime.contains('/') => media.mime = Some(mime),
            _ => {}
        }

        match media.mime {
            Some(ref mime) if mime.starts_with("video") => {
                match image.mp4.take() {
                    Some(mp4) if mime.ends_with("webm") => {
                        let mut alt = media.clone();
                        alt.mime = Some(SmolStr::new_inline("video/mp4"));
                        alt.url = add_noredirect(mp4).into();
                        media.alternate = Some(alt);
                    }
                    _ => {}
                }

                embed.video = Some(media);
            }
            _ => embed.img = Some(media),
        }

        static IMGUR_PROVIDER: Lazy<EmbedProvider> = Lazy::new(|| {
            let mut provider = EmbedProvider::default();

            provider.name = Some(SmolStr::new_inline("imgur"));
            provider.url = Some(SmolStr::new_inline("https://imgur.com"));
            provider.icon = Some(BoxedEmbedMedia::default().with_url("https://s.imgur.com/images/favicon.png"));

            provider
        });

        embed.provider = IMGUR_PROVIDER.clone();

        if match (data.nsfw, data.ad_config) {
            (Some(true), _) => true,
            (_, Some(ref ad_config)) => ad_config.nsfw_score > 0.75,
            _ => false,
        } {
            embed.flags |= EmbedFlags::ADULT;
        }

        embed.url = Some({
            let mut origin = url.origin().ascii_serialization();
            origin += url.path();
            origin.into()
        });

        embed.title = data.title;
        embed.description = data.description;

        embed.color = Some(0x85bf25);

        if data.images_count > 1 {
            let rem = data.images_count - 1;
            embed.footer = Some(EmbedFooter {
                text: smol_str::format_smolstr!(
                    "and {rem} more {}",
                    match rem {
                        1 => "file",
                        _ => "files",
                    }
                ),
                icon: None,
            });
        }

        // 4-hour expire
        Ok(generic::finalize_embed(state, embed, Some(60 * 60 * 4)))
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum ImgurResult {
    Success {
        success: monostate::MustBe!(true),

        #[serde(default)]
        data: Option<ImgurData>,
    },
    Failure {},
}

#[derive(Debug, serde::Deserialize)]
pub struct ImgurData {
    #[serde(default)]
    pub ad_config: Option<ImgurAdConfig>,

    #[serde(default)]
    pub images_count: usize,

    #[serde(flatten)]
    pub kind: ImgurDataKind,

    #[serde(default)]
    pub cover: Option<SmolStr>,

    #[serde(default)]
    pub title: Option<SmolStr>,

    #[serde(default)]
    pub description: Option<SmolStr>,

    #[serde(default)]
    pub nsfw: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum ImgurDataKind {
    Gallery {
        is_gallery: monostate::MustBe!(true),

        #[serde(default)]
        images: Vec<ImgurImageData>,
    },
    Album {
        is_album: monostate::MustBe!(true),

        #[serde(default)]
        images: Vec<ImgurImageData>,
    },
    Image {
        #[serde(flatten)]
        image: ImgurImageData,
    },
}

#[derive(Debug, serde::Deserialize)]
pub struct ImgurImageData {
    pub id: SmolStr,

    #[serde(default, rename = "type")]
    pub mime: Option<SmolStr>,

    #[serde(default)]
    pub width: Option<i32>,
    #[serde(default)]
    pub height: Option<i32>,

    pub link: String,

    #[serde(default)]
    pub mp4: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ImgurAdConfig {
    #[serde(default)]
    pub nsfw_score: f32,
}

fn add_noredirect(mut url: String) -> String {
    if !url.ends_with("?noredirect") {
        url += "?noredirect";
    }
    url
}
