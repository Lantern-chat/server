extern crate client_sdk as sdk;

pub mod config;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::post,
    Json,
};
use config::Config;
use embed_parser::{
    embed,
    oembed::{OEmbed, OEmbedFormat, OEmbedLink},
};
use futures_util::FutureExt;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Method,
};
use std::{net::SocketAddr, str::FromStr, sync::Arc};

use sdk::models::*;

use hmac::{digest::Key, Mac};
type Hmac = hmac::SimpleHmac<sha1::Sha1>;

struct WorkerState {
    config: Config,
    signing_key: Key<Hmac>,
    client: reqwest::Client,
}

use base64::engine::{general_purpose::URL_SAFE_NO_PAD, Engine};

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Unable to use .env");

    let config = {
        let config_path = std::env::var("EMBEDW_CONFIG_PATH").unwrap_or_else(|_| "./config.toml".to_owned());

        let config_file = std::fs::read_to_string(config_path).expect("Unable to read config file");

        let parsed: config::ParsedConfig = toml::de::from_str(&config_file).expect("Unable to parse config file");

        parsed.build().expect("Unable to build config")
    };

    let state = Arc::new(WorkerState {
        signing_key: {
            let hex_key = std::env::var("CAMO_SIGNING_KEY").expect("CAMO_SIGNING_KEY not found");
            let mut raw_key = Key::<Hmac>::default();
            // keys are allowed to be shorter than the entire raw key. Will be padded internally.
            hex::decode_to_slice(&hex_key, &mut raw_key[..hex_key.len() / 2])
                .expect("Could not parse signing key!");

            raw_key
        },
        client: {
            reqwest::ClientBuilder::new()
                .default_headers({
                    let mut headers = HeaderMap::new();

                    headers.insert(
                       HeaderName::from_static("accept"),
                       HeaderValue::from_static(
                           "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
                       ),
                    );

                    headers.insert(HeaderName::from_static("dnt"), HeaderValue::from_static("1"));
                    headers.insert(
                        HeaderName::from_static("user-agent"),
                        HeaderValue::from_static("Lantern/1.0 (bot; +https://github.com/Lantern-chat)"),
                    );

                    headers
                })
                .gzip(true)
                .deflate(true)
                .brotli(true)
                .redirect(reqwest::redirect::Policy::limited(config.parsed.max_redirects as usize))
                .connect_timeout(std::time::Duration::from_millis(config.parsed.timeout))
                .danger_accept_invalid_certs(false)
                .http2_adaptive_window(true)
                .build()
                .expect("Unable to build primary client")
        },
        config,
    });

    let addr = std::env::var("EMBEDW_BIND_ADDRESS").expect("EMBEDW_BIND_ADDRESS not found");
    let addr = SocketAddr::from_str(&addr).expect("Unable to parse bind address");

    println!("Starting...");

    axum::Server::bind(&addr)
        .serve(post(root).with_state(state).into_make_service())
        .with_graceful_shutdown(tokio::signal::ctrl_c().map(|_| ()))
        .await
        .expect("Unable to run embed-worker");

    println!("Goodbye.");
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Params {
    #[serde(rename = "l")]
    pub lang: Option<String>,
}

async fn root(
    State(state): State<Arc<WorkerState>>,
    Query(params): Query<Params>,
    body: String,
) -> Result<Json<(Timestamp, Embed)>, (StatusCode, String)> {
    let url = body; // to avoid confusion

    match inner(state, url, params).await {
        Ok(value) => Ok(Json(value)),
        Err(e) => Err({
            let code = match e {
                Error::InvalidUrl => StatusCode::BAD_REQUEST,
                Error::InvalidMimeType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
                Error::Failure(code) => code,
                Error::ReqwestError(ref e) => match e.status() {
                    Some(status) => status,
                    None if e.is_connect() => StatusCode::REQUEST_TIMEOUT,
                    None => StatusCode::INTERNAL_SERVER_ERROR,
                },
                Error::JsonError(_) | Error::XMLError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            };

            let msg = if code.is_server_error() { "Internal Server Error".to_owned() } else { e.to_string() };

            (code, msg)
        }),
    }
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Invalid URL")]
    InvalidUrl,

    #[error("Failure: {0}")]
    Failure(StatusCode),

    #[error("Invalid MIME Type")]
    InvalidMimeType,

    #[error("JSON Error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("XML Error: {0}")]
    XMLError(#[from] quick_xml::de::DeError),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
}

async fn inner(state: Arc<WorkerState>, url: String, params: Params) -> Result<(Timestamp, Embed), Error> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(Error::InvalidUrl);
    }

    let (https, root, domain) = embed_parser::utils::url_root(&url);

    let mut resp = retry_request(2, || {
        let mut req = state.client.get(url.as_str());

        if let Some(user_agent) = state.config.user_agent(domain) {
            req = req.header(HeaderName::from_static("user-agent"), user_agent);
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
    let mut max_age = 0;

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
        if let Ok(o) = fetch_oembed(&state, link, domain).await {
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
                let extra = embed::parse_meta_to_embed(&mut embed, &headers);

                match extra.link {
                    Some(link) if oembed.is_none() => {
                        if let Ok(o) = fetch_oembed(&state, &link, domain).await {
                            oembed = o;
                        }
                    }
                    _ => {}
                }

                max_age = extra.max_age;
            }

            drop(html); // ensure it lives long enough
        } else {
            let mut media = Box::new(EmbedMedia {
                url: url.as_str().into(),
                mime: Some(mime.into()),
                ..EmbedMedia::default()
            });

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
        let extra = embed::parse_oembed_to_embed(&mut embed, oembed);

        max_age = extra.max_age;
    }

    embed_parser::quirks::resolve_relative(root, https, &mut embed);

    if state.config.parsed.resolve_media {
        resolve_images(&state.client, &mut embed).await?;
    }

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

    embed_parser::quirks::fix_embed(&mut embed);

    embed.visit_media_mut(|media| {
        let sig = Hmac::new(&state.signing_key)
            .chain_update(&*media.url)
            .finalize()
            .into_bytes();

        let mut buf = [0; 27];
        if let Ok(27) = URL_SAFE_NO_PAD.encode_slice(sig, &mut buf) {
            use sdk::util::fixed::FixedStr;

            media.signature = Some(FixedStr::new(unsafe { std::str::from_utf8_unchecked(&buf) }));
        }
    });

    if let Some(site) = state.config.find_site(domain) {
        embed.color = site.color.or(embed.color);
    }

    // compute expirey
    let expires = {
        use iso8601_timestamp::Duration;

        embed.ts = Timestamp::now_utc();

        // limit max_age to 1 month, minimum 15 minutes
        embed
            .ts
            .checked_add(Duration::seconds(max_age.min(60 * 60 * 24 * 30).max(60 * 15) as i64))
            .unwrap()
    };

    Ok((expires, sdk::models::Embed::V1(embed)))
}

async fn fetch_oembed<'a>(
    state: &WorkerState,
    link: &OEmbedLink<'a>,
    domain: &str,
) -> Result<Option<OEmbed>, Error> {
    if state.config.skip_oembed(domain).is_match() {
        return Ok(None);
    }

    let body = state.client.get(&*link.url).send().await?.bytes().await?;

    Ok(Some(match link.format {
        OEmbedFormat::JSON => serde_json::de::from_slice(&body)?,
        OEmbedFormat::XML => quick_xml::de::from_reader(&*body)?,
    }))
}

async fn read_head<'a>(
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

async fn read_bytes<'a>(resp: &'a mut reqwest::Response, bytes: &'a mut Vec<u8>, max: usize) -> Result<(), Error> {
    while let Some(chunk) = resp.chunk().await? {
        bytes.extend(&chunk);

        if bytes.len() > max {
            break;
        }
    }

    Ok(())
}

async fn resolve_images(client: &reqwest::Client, embed: &mut EmbedV1) -> Result<(), Error> {
    use futures_util::stream::{FuturesUnordered, StreamExt};

    let f = FuturesUnordered::new();

    if let Some(ref mut media) = embed.img {
        f.push(resolve_media(client, &mut *media, false));
    }

    if let Some(ref mut media) = embed.thumb {
        f.push(resolve_media(client, &mut *media, false));
    }

    // assert this is html
    if let Some(ref mut media) = embed.obj {
        f.push(resolve_media(client, &mut *media, true));
    }

    if let Some(ref mut footer) = embed.footer {
        if let Some(ref mut media) = footer.icon {
            f.push(resolve_media(client, &mut *media, false));
        }
    }

    if let Some(ref mut author) = embed.author {
        if let Some(ref mut media) = author.icon {
            f.push(resolve_media(client, &mut *media, false));
        }
    }

    for field in &mut embed.fields {
        if let Some(ref mut media) = field.img {
            f.push(resolve_media(client, &mut *media, true));
        }
    }

    let _ = f.count().await;

    Ok(())
}

async fn retry_request<F>(max_attempts: u8, mut make_request: F) -> Result<reqwest::Response, Error>
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

async fn resolve_media(client: &reqwest::Client, media: &mut EmbedMedia, head: bool) -> Result<(), Error> {
    // already has dimensions
    if !head && !matches!((media.width, media.height), (None, None)) {
        return Ok(());
    }

    // TODO: Remove when relative paths are handled
    if media.url.starts_with('.') {
        return Ok(());
    }

    let mut resp = retry_request(2, || {
        client.request(if head { Method::HEAD } else { Method::GET }, &*media.url)
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
