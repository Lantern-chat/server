use std::borrow::Cow;

use super::prelude::*;

use ego_tree::iter::Edge;
use scraper::{CaseSensitivity::AsciiCaseInsensitive, ElementRef, Node, Selector};

macro_rules! selector {
    ($e: expr) => {{
        use once_cell::sync::Lazy;
        static SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse($e).unwrap());
        &*SELECTOR
    }};
}

pub struct FurAffinityExtractorFactory;

impl ExtractorFactory for FurAffinityExtractorFactory {
    fn create(&self, config: &Config) -> Result<Option<Box<dyn Extractor>>, ConfigError> {
        let Some(extractor) = config.parsed.extractors.get("furaffinity") else {
            return Ok(None);
        };

        let Some(a) = extractor.get("a") else {
            return Err(ConfigError::MissingExtractorField("furaffinity.a"));
        };

        let Some(b) = extractor.get("b") else {
            return Err(ConfigError::MissingExtractorField("furaffinity.b"));
        };

        let Some(ua) = config.parsed.user_agents.get("%browser") else {
            return Err(ConfigError::InvalidUserAgent("%browser not found".to_owned()));
        };

        let Ok(cookie) = HeaderValue::try_from(format!("b={b}; a={a}")) else {
            return Err(ConfigError::InvalidExtractorField("furaffinity.(a|b)"));
        };

        Ok(Some(Box::new(FurAffinityExtractor {
            cookie,
            user_agent: ua.0.clone(),
        })))
    }
}

#[derive(Debug)]
pub struct FurAffinityExtractor {
    pub cookie: HeaderValue,
    pub user_agent: HeaderValue,
}

#[async_trait::async_trait]
impl Extractor for FurAffinityExtractor {
    fn matches(&self, url: &Url) -> bool {
        matches!(url.domain(), Some("furaffinity.net" | "www.furaffinity.net")) && url.path().starts_with("/view/")
    }

    async fn extract(&self, state: Arc<WorkerState>, url: Url, params: Params) -> Result<EmbedWithExpire, Error> {
        let html = state
            .client
            .get(url.clone())
            .header(HeaderName::from_static("cookie"), &self.cookie)
            .header(HeaderName::from_static("user-agent"), &self.user_agent)
            .send()
            .await?
            .text()
            .await?;

        let mut embed = parse_html(&html, &url)?;

        generic::resolve_images(&state, &None, &mut embed).await?;

        Ok(generic::finalize_embed(state, embed, Some(60 * 60 * 4)))
    }
}

fn trim_nl(t: &str) -> &str {
    t.trim_matches(|c: char| matches!(c, '\r' | '\n'))
}

fn fix_relative_scheme(url: &str) -> Cow<str> {
    match url.strip_prefix("//") {
        Some(url) => Cow::Owned(format!("https://{url}")),
        None => Cow::Borrowed(url),
    }
}

fn parse_html(html: &str, url: &Url) -> Result<EmbedV1, Error> {
    let doc = scraper::Html::parse_document(html);

    let mut embed = EmbedV1::default();

    #[derive(Debug, PartialEq, Eq)]
    enum Kind {
        Image,
        Video,
        Audio,
        Unsupported,
    }

    // find submission and parse media nodes
    if let Some(node) = doc.select(selector!("div.submission-area")).next() {
        let mut src = None;
        let mut alt = None;
        let mut kind = Kind::Unsupported;

        let use_thumbnail = node.value().has_class("submission-writing", AsciiCaseInsensitive);

        for e in node.traverse() {
            let Edge::Open(node) = e else { continue; };
            let Node::Element(el) = node.value() else { continue; };
            kind = match el.name() {
                "img" => Kind::Image,
                "audio" => Kind::Audio,
                "vid" => Kind::Video,
                "object" => break,
                _ => continue,
            };

            src = el.attr("src");
            alt = el.attr("alt");
            break;
        }

        match src {
            Some(src) if kind != Kind::Unsupported => {
                let mut media = BoxedEmbedMedia::default();

                media.url = fix_relative_scheme(src).into();
                media.description = alt.map(SmolStr::new);

                match kind {
                    Kind::Image if use_thumbnail => embed.thumb = Some(media),
                    Kind::Image => embed.img = Some(media),
                    Kind::Video => embed.video = Some(media),
                    Kind::Audio => embed.audio = Some(media),
                    _ => {}
                }
            }
            _ => {}
        }
    }

    // aggregate description text
    if let Some(node) = doc.select(selector!("div.submission-description")).next() {
        let mut description = String::new();

        for e in node.traverse() {
            let Edge::Open(node) = e else { continue; };
            description += match node.value() {
                Node::Text(t) => trim_nl(t).trim_start(),
                Node::Element(el) => match el.name() {
                    "br" if !description.ends_with("\n\n") => "\n",
                    "img" => match el.attr("alt") {
                        Some(alt_text) => alt_text,
                        None => continue,
                    },
                    _ => continue,
                },
                _ => continue,
            };
        }

        let truncate_to = description.trim_end().len();
        description.truncate(truncate_to);

        embed.description = Some(description.into());
    }

    let mut author = EmbedAuthor::default();

    if let Some(node) = doc.select(selector!("div.submission-title")).next() {
        let mut title = String::new();
        for chunk in node.text() {
            title += chunk;
        }
        embed.title = Some(title.into());

        for sibling in node.next_siblings() {
            let Node::Element(el) = sibling.value() else { continue; };

            match el.attr("href") {
                // <a href="/user/AUTHOR">
                Some(href) if href.starts_with("/user/") => {
                    author.url = Some(format!("https://www.furaffinity.net{href}").into());

                    // <strong>AUTHOR NAME
                    if let Some(el) = ElementRef::wrap(sibling) {
                        let mut name = String::new();
                        for chunk in el.text() {
                            name += chunk;
                        }
                        author.name = name.into();
                    }

                    break;
                }
                _ => {}
            }
        }
    }

    if let Some(node) = doc.select(selector!("img.submission-user-icon")).next() {
        if let Some(src) = node.value().attr("src") {
            let mut media = BoxedEmbedMedia::default();
            media.url = fix_relative_scheme(src).into();
            author.icon = Some(media);
        }
    }

    embed.author = Some(author);

    if let Some(node) = doc.select(selector!("span.rating-box")).next() {
        if !node.value().has_class("general", AsciiCaseInsensitive) {
            embed.flags |= EmbedFlags::ADULT;
        }
    }

    embed.url = Some({
        let mut origin = url.origin().ascii_serialization();
        origin += url.path();
        origin.into()
    });

    embed.color = Some(0xadd8f5);

    static FA_PROVIDER: Lazy<EmbedProvider> = Lazy::new(|| {
        let mut provider = EmbedProvider::default();

        provider.name = Some(SmolStr::new_inline("FurAffinity"));
        provider.url = Some(SmolStr::new("https://www.furaffinity.net"));
        provider.icon =
            Some(BoxedEmbedMedia::default().with_url("https://www.furaffinity.net/themes/beta/img/favicon.ico"));

        provider
    });

    embed.provider = FA_PROVIDER.clone();

    Ok(embed)
}
