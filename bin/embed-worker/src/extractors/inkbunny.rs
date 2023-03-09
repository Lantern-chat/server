use std::collections::VecDeque;

use super::prelude::*;

use embed_parser::oembed::Integer64;

pub struct InkbunnyExtractorFactory;

use arc_swap::ArcSwapOption;

#[derive(Default, Debug)]
pub struct InkbunnyExtractor {
    pub username: Option<String>,
    pub password: Option<String>,

    pub session_id: ArcSwapOption<String>,
}

impl ExtractorFactory for InkbunnyExtractorFactory {
    fn create(&self, config: &Config) -> Result<Option<Box<dyn Extractor>>, ConfigError> {
        let Some(extractor) = config.parsed.extractors.get("inkbunny") else {
            return Ok(None);
        };

        let mut ibe = Box::<InkbunnyExtractor>::default();

        match (
            extractor.get("session_id"),
            extractor.get("username"),
            extractor.get("password"),
        ) {
            (Some(session_id), _, _) => ibe.session_id.store(Some(Arc::new(session_id.clone()))),
            (None, Some(username), Some(password)) => {
                ibe.username = Some(username.clone());
                ibe.password = Some(password.clone());
            }
            (None, None, None) => {
                return Err(ConfigError::MissingExtractorField(
                    "inkbunny.(session_id|username|password)",
                ));
            }
            (None, Some(_), None) => {
                return Err(ConfigError::MissingExtractorField("inkbunny.password"));
            }
            (None, None, Some(_)) => {
                return Err(ConfigError::MissingExtractorField("inkbunny.username"));
            }
        }

        Ok(Some(ibe))
    }
}

#[async_trait::async_trait]
impl Extractor for InkbunnyExtractor {
    fn matches(&self, url: &Url) -> bool {
        matches!(url.domain(), Some("inkbunny.net")) && url.path().starts_with("/s/")
    }

    async fn setup(&self, state: Arc<WorkerState>) -> Result<(), Error> {
        if self.session_id.load().is_some() {
            return Ok(());
        }

        let login_uri = match (&self.username, &self.password) {
            (Some(username), Some(password)) => {
                println!("Logging into Inkbunny as {username}!");

                format!(
                    "https://inkbunny.net/api_login.php?output_mode=json&username={username}&password={password}"
                )
            }
            _ => return Err(Error::Failure(StatusCode::UNAUTHORIZED)),
        };

        let resp = state
            .client
            .post(login_uri)
            .send()
            .await?
            .json::<InkbunnyLoginResult>()
            .await?;

        let InkbunnyLoginResult::Success { sid } = resp else {
            return Err(Error::Failure(StatusCode::UNAUTHORIZED));
        };

        self.session_id.store(Some(Arc::new(sid)));

        Ok(())
    }

    async fn extract(&self, state: Arc<WorkerState>, url: Url, params: Params) -> Result<EmbedWithExpire, Error> {
        let Some(image_id) = url.path_segments().unwrap().nth(1) else {
            return Err(Error::Failure(StatusCode::NOT_FOUND));
        };

        // avoid injection of multiple ids
        let Some(image_id) = image_id.split(|c: char| !c.is_ascii_alphanumeric()).next() else {
            return Err(Error::Failure(StatusCode::NOT_FOUND));
        };

        let sid_guard = self.session_id.load();
        let sid = sid_guard.as_ref().unwrap();
        let api_uri = format!("https://inkbunny.net/api_submissions.php?output_mode=json&show_description=yes&sid={sid}&submission_ids={image_id}");
        drop(sid_guard);

        let resp = state.client.get(api_uri).send().await?.json().await?;

        let InkbunnyResult::Success { submissions: [mut submission] } = resp else {
            return Err(Error::Failure(StatusCode::NOT_FOUND));
        };

        // get first file
        let Some(file) = submission.files.pop_front() else {
            return Err(Error::Failure(StatusCode::NOT_FOUND));
        };

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Kind {
            Text,
            Video,
            Image,
            Audio,
        }

        let kind = match file.mimetype.as_ref().and_then(|mime| mime.get(..5)) {
            Some("image") => Kind::Image,
            Some("video") => Kind::Video,
            Some("audio") => Kind::Audio,
            // text or application/whatever
            Some("text/" | "appli") => Kind::Text,
            _ => return Err(Error::Failure(StatusCode::UNSUPPORTED_MEDIA_TYPE)),
        };

        let mut embed = EmbedV1::default();

        if !submission.files.is_empty() {
            embed.footer = Some(EmbedFooter {
                text: smol_str::format_smolstr!(
                    "and {} more {}",
                    submission.files.len(),
                    match submission.files.len() {
                        1 => "file",
                        _ => "files",
                    }
                ),
                icon: None,
            });
        }

        if kind != Kind::Text {
            let mut media = BoxedEmbedMedia::default();

            media.mime = file.mimetype;

            let very_large = match (file.full_size_x, file.full_size_y) {
                ((Some(Integer64(w)), Some(Integer64(h)))) => w > 4096 || h > 4096,
                _ => false,
            };

            if kind != Kind::Image || !very_large {
                media.url = file.file_url_full;
                media.width = file.full_size_x.map(|x| x.0 as _);
                media.height = file.full_size_y.map(|x| x.0 as _);
                media.description = file.file_name;
            } else {
                media.url = file.file_url_screen;
                media.width = file.screen_size_x.map(|x| x.0 as _);
                media.height = file.screen_size_y.map(|x| x.0 as _);
                media.description = file.file_name;
            }

            match kind {
                Kind::Image => embed.img = Some(media),
                Kind::Video => embed.video = Some(media),
                Kind::Audio => embed.audio = Some(media),
                _ => {}
            }
        }

        if let Some(thumb_url) = file.thumbnail_url_huge {
            embed.thumb = Some({
                let mut media = BoxedEmbedMedia::default();
                media.url = thumb_url;
                media.width = file.thumb_huge_x.map(|x| x.0 as _);
                media.height = file.thumb_huge_y.map(|x| x.0 as _);
                media
            });
        }

        embed.color = Some(0x73d216);
        embed.provider.name = Some(SmolStr::new_inline("Inkbunny"));
        embed.provider.url = Some(SmolStr::new_inline("https://inkbunny.net"));
        embed.provider.icon =
            Some(BoxedEmbedMedia::default().with_url("https://va.ib.metapix.net/images80/favicon.ico"));

        embed.author = Some({
            let mut author = EmbedAuthor::default();
            if let Some(icon) = submission.user_icon_url_small {
                author.icon = BoxedEmbedMedia::default().with_url(icon).into();
            }

            author.name = submission.username;
            author.url = Some(smol_str::format_smolstr!("https://inkbunny.net/{}", author.name));

            author
        });

        embed.title = submission.title;
        embed.description = submission.description;

        embed.url = Some({
            let mut origin = url.origin().ascii_serialization();
            origin += url.path();
            origin.into()
        });

        if submission.rating_id.0 != 0 {
            embed.flags |= EmbedFlags::ADULT;
        }

        if embed.img.is_some() || embed.video.is_some() {
            embed.thumb = None;
        }

        // 4-hour expire
        Ok(generic::finalize_embed(state, embed, Some(60 * 60 * 4)))
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum InkbunnyLoginResult {
    Success { sid: String },
    Failure {},
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum InkbunnyResult {
    Success { submissions: [InkbunnySubmission; 1] },
    Error {},
}

#[derive(Debug, serde::Deserialize)]
pub struct InkbunnySubmission {
    pub username: SmolStr,

    #[serde(default)]
    pub title: Option<SmolStr>,
    #[serde(default)]
    pub description: Option<SmolStr>,

    #[serde(default)]
    pub user_icon_url_small: Option<SmolStr>,

    #[serde(default)]
    pub files: VecDeque<InkbunnyFile>,

    pub rating_id: Integer64,
}

#[derive(Debug, serde::Deserialize)]
pub struct InkbunnyFile {
    #[serde(default)]
    pub file_name: Option<SmolStr>,
    #[serde(default)]
    pub mimetype: Option<SmolStr>,

    pub file_url_full: SmolStr,
    #[serde(default)]
    pub full_size_x: Option<Integer64>,
    #[serde(default)]
    pub full_size_y: Option<Integer64>,

    pub file_url_screen: SmolStr,
    #[serde(default)]
    pub screen_size_x: Option<Integer64>,
    #[serde(default)]
    pub screen_size_y: Option<Integer64>,

    #[serde(default)]
    pub thumbnail_url_huge: Option<SmolStr>,
    #[serde(default)]
    pub thumb_huge_x: Option<Integer64>,
    #[serde(default)]
    pub thumb_huge_y: Option<Integer64>,
}
