use super::prelude::*;

#[derive(Debug)]
pub struct GenericExtractor;

impl ExtractorFactory for GenericExtractor {
    fn create(&self, _config: &Config) -> Result<Option<Box<dyn Extractor>>, ConfigError> {
        Ok(Some(Box::new(GenericExtractor)))
    }
}

#[async_trait::async_trait]
impl Extractor for GenericExtractor {
    fn matches(&self, _: &url::Url) -> bool {
        true
    }

    async fn extract(
        &self,
        state: Arc<WorkerState>,
        url: url::Url,
        params: Params,
    ) -> Result<EmbedWithExpire, Error> {
        if !url.scheme().starts_with("http") {
            return Err(Error::InvalidUrl);
        }

        let site = url.domain().and_then(|domain| state.config.find_site(domain));

        let mut resp = retry_request(2, || {
            let mut req = state.client.get(url.as_str());

            if let Some(ref site) = site {
                req = site.add_headers(&state.config, req);
            }

            if let Some(ref lang) = params.lang {
                req = req.header(HeaderName::from_static("accept-language"), format!("{lang};q=0.5"));
            }

            req
        })
        .await?;

        if !resp.status().is_success() {
            return Err(Error::Failure(resp.status()));
        }

        let mut embed = sdk::models::EmbedV1::default();
        let mut oembed: Option<OEmbed> = None;
        let mut max_age = None;

        if let Some(rating) = resp.headers().get(HeaderName::from_static("rating")) {
            if embed_parser::regexes::ADULT_RATING.is_match(rating.as_bytes()) {
                embed.flags |= EmbedFlags::ADULT;
            }
        }

        let links = resp
            .headers()
            .get("link")
            .and_then(|h| h.to_str().ok())
            .map(embed_parser::oembed::parse_link_header);

        embed.url = Some(url.as_str().into());

        if let Some(link) = links.as_ref().and_then(|l| l.first()) {
            if let Ok(o) = fetch_oembed(&state, link, url.domain()).await {
                oembed = o;
            }
        }

        drop(links);

        if let Some(mime) = resp.headers().get("content-type").and_then(|h| h.to_str().ok()) {
            let Some(mime) = mime.split(';').next() else {
            return Err(Error::InvalidMimeType);
        };

            if mime == "text/html" {
                let mut html = Vec::with_capacity(512);
                if let Some(headers) = read_head(&mut resp, &mut html).await? {
                    let extra = embed_parser::embed::parse_meta_to_embed(&mut embed, &headers);

                    match extra.link {
                        Some(link) if oembed.is_none() => {
                            if let Ok(o) = fetch_oembed(&state, &link, url.domain()).await {
                                oembed = o;
                            }
                        }
                        _ => {}
                    }

                    max_age = extra.max_age;
                }

                drop(html); // ensure it lives long enough
            } else {
                let mut media = BoxedEmbedMedia::default();
                media.url = url.as_str().into();
                media.mime = Some(mime.into());

                match mime.get(0..5) {
                    Some("image") => {
                        let mut bytes = Vec::with_capacity(512);

                        if let Ok(_) = read_bytes(&mut resp, &mut bytes, 1024 * 1024).await {
                            if let Ok(image_size) = imagesize::blob_size(&bytes) {
                                media.width = Some(image_size.width as _);
                                media.height = Some(image_size.height as _);
                            }
                        }

                        embed.ty = EmbedType::Img;
                        embed.img = Some(media);
                    }
                    Some("video") => {
                        embed.ty = EmbedType::Vid;
                        embed.video = Some(media);
                    }
                    Some("audio") => {
                        embed.ty = EmbedType::Audio;
                        embed.audio = Some(media);
                    }
                    _ => {}
                }
            }
        }

        if let Some(oembed) = oembed {
            let extra = embed_parser::embed::parse_oembed_to_embed(&mut embed, oembed);

            max_age = extra.max_age;
        }

        embed_parser::quirks::resolve_relative(&url, &mut embed);

        if state.config.parsed.resolve_media {
            resolve_images(&state, &site, &mut embed).await?;
        }

        if let Some(domain) = url.domain() {
            if !state.config.allow_html(domain).is_match() {
                embed.obj = None;

                if let Some(ref vid) = embed.video {
                    if let Some(ref mime) = vid.mime {
                        if mime.starts_with("text/html") {
                            embed.video = None;
                        }
                    }
                }
            }

            if let Some(site) = state.config.find_site(domain) {
                embed.color = site.color.or(embed.color);
            }
        }

        Ok(finalize_embed(state, embed, max_age))
    }
}

pub fn finalize_embed(state: Arc<WorkerState>, mut embed: EmbedV1, max_age: Option<u64>) -> EmbedWithExpire {
    embed_parser::quirks::fix_embed(&mut embed);

    embed.visit_media_mut(|media| {
        media.signature = state.sign(&media.url);
    });

    let expires = {
        use iso8601_timestamp::{Duration, Timestamp};

        embed.ts = Timestamp::now_utc();

        // limit max_age to 1 month, minimum 15 minutes
        embed
            .ts
            .checked_add(Duration::seconds(
                max_age.unwrap_or(60 * 15).min(60 * 60 * 24 * 30).max(60 * 15) as i64,
            ))
            .unwrap()
    };

    (expires, sdk::models::Embed::V1(embed))
}

pub async fn fetch_oembed<'a>(
    state: &WorkerState,
    link: &OEmbedLink<'a>,
    domain: Option<&str>,
) -> Result<Option<OEmbed>, Error> {
    if let Some(domain) = domain {
        if state.config.skip_oembed(domain).is_match() {
            return Ok(None);
        }
    }

    let body = state.client.get(&*link.url).send().await?.bytes().await?;

    Ok(Some(match link.format {
        OEmbedFormat::JSON => serde_json::de::from_slice(&body)?,
        OEmbedFormat::XML => quick_xml::de::from_reader(&*body)?,
    }))
}

pub async fn read_head<'a>(
    resp: &'a mut reqwest::Response,
    html: &'a mut Vec<u8>,
) -> Result<Option<embed_parser::html::HeaderList<'a>>, Error> {
    while let Some(chunk) = resp.chunk().await? {
        html.extend(&chunk);

        if memchr::memmem::rfind(html, b"</body").is_some() {
            break;
        }

        // 1MB of HTML downloaded, assume it's a broken page or DoS attack and don't bother with more
        if html.len() > (1024 * 1024) {
            break;
        }
    }

    if let std::borrow::Cow::Owned(new_html) = String::from_utf8_lossy(html) {
        *html = new_html.into();
    }

    // SAFETY: Just converted it to lossy utf8, it's fine
    Ok(embed_parser::html::parse_meta(unsafe {
        std::str::from_utf8_unchecked(html)
    }))
}

pub async fn read_bytes<'a>(
    resp: &'a mut reqwest::Response,
    bytes: &'a mut Vec<u8>,
    max: usize,
) -> Result<(), Error> {
    while let Some(chunk) = resp.chunk().await? {
        bytes.extend(&chunk);

        if bytes.len() > max {
            break;
        }
    }

    Ok(())
}

pub async fn resolve_images(
    state: &WorkerState,
    site: &Option<Arc<Site>>,
    embed: &mut sdk::models::EmbedV1,
) -> Result<(), Error> {
    use futures_util::stream::{FuturesUnordered, StreamExt};

    let f = FuturesUnordered::new();

    if let Some(ref mut media) = embed.img {
        f.push(resolve_media(state, site, &mut *media, false));
    }

    if let Some(ref mut media) = embed.thumb {
        f.push(resolve_media(state, site, &mut *media, false));
    }

    // assert this is html
    if let Some(ref mut media) = embed.obj {
        f.push(resolve_media(state, site, &mut *media, true));
    }

    if let Some(ref mut footer) = embed.footer {
        if let Some(ref mut media) = footer.icon {
            f.push(resolve_media(state, site, &mut *media, false));
        }
    }

    if let Some(ref mut author) = embed.author {
        if let Some(ref mut media) = author.icon {
            f.push(resolve_media(state, site, &mut *media, false));
        }
    }

    for field in &mut embed.fields {
        if let Some(ref mut media) = field.img {
            f.push(resolve_media(state, site, &mut *media, true));
        }
    }

    let _ = f.count().await;

    Ok(())
}

pub async fn retry_request<F>(max_attempts: u8, mut make_request: F) -> Result<reqwest::Response, Error>
where
    F: FnMut() -> reqwest::RequestBuilder,
{
    let mut req = make_request().send().boxed();
    let mut attempts = 1;

    loop {
        match req.await {
            Ok(resp) => break Ok(resp),
            Err(e) if e.is_timeout() && attempts < max_attempts => {
                attempts += 1;
                req = make_request().send().boxed();
            }
            Err(e) => return Err(e.into()),
        }
    }
}

pub async fn resolve_media(
    state: &WorkerState,
    site: &Option<Arc<Site>>,
    media: &mut sdk::models::EmbedMedia,
    head: bool,
) -> Result<(), Error> {
    // already has dimensions
    if !head && !matches!((media.width, media.height), (None, None)) {
        return Ok(());
    }

    // TODO: Remove when relative paths are handled
    if media.url.starts_with('.') {
        return Ok(());
    }

    let mut resp = retry_request(2, || {
        let mut req = state
            .client
            .request(if head { Method::HEAD } else { Method::GET }, &*media.url);

        if let Some(ref site) = site {
            req = site.add_headers(&state.config, req);
        }

        req
    })
    .await?;

    if let Some(mime) = resp.headers().get("content-type").and_then(|h| h.to_str().ok()) {
        media.mime = Some(mime.into());

        if !head && mime.starts_with("image") {
            let mut bytes = Vec::with_capacity(512);

            if let Ok(_) = read_bytes(&mut resp, &mut bytes, 1024 * 512).await {
                if let Ok(image_size) = imagesize::blob_size(&bytes) {
                    media.width = Some(image_size.width as _);
                    media.height = Some(image_size.height as _);
                }
            }
        }
    }

    Ok(())
}
