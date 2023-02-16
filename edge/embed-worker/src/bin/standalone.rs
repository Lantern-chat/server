extern crate client_sdk as sdk;

use embed_parser::{
    embed,
    html::Header,
    oembed::{OEmbed, OEmbedFormat, OEmbedLink},
};
use futures_util::FutureExt;
use reqwest::Client;
use sdk::models::*;

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use std::{net::SocketAddr, str::FromStr, sync::Arc};

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Unable to use .env");

    let state = Arc::new(
        reqwest::ClientBuilder::new()
            .user_agent("Mozilla/5.0 (compatible; Lantern Embed Worker; +https://lantern.chat)")
            .gzip(true)
            .deflate(true)
            .brotli(true)
            .redirect(reqwest::redirect::Policy::limited(1))
            .connect_timeout(std::time::Duration::from_secs(10))
            .danger_accept_invalid_certs(false)
            .http2_adaptive_window(true)
            .build()
            .expect("Unable to build primary client"),
    );

    let addr = std::env::var("EMBEDW_BIND_ADDRESS").expect("EMBEDW_BIND_ADDRESS not found");
    let addr = SocketAddr::from_str(&addr).expect("Unable to parse bind address");

    axum::Server::bind(&addr)
        .serve(Router::new().fallback(post(root)).with_state(state).into_make_service())
        .with_graceful_shutdown(tokio::signal::ctrl_c().map(|_| ()))
        .await
        .expect("Unable to run embed-worker");
}

async fn root(
    State(client): State<Arc<Client>>,
    body: String,
) -> Result<Json<(Timestamp, Embed)>, (StatusCode, String)> {
    let url = body; // to avoid confusion

    match inner(client, url).await {
        Ok(value) => Ok(Json(value)),
        Err(e) => Err({
            let code = match e {
                Error::InvalidUrl => StatusCode::BAD_REQUEST,
                Error::InvalidMimeType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
                Error::Failure(code) => code,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
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

    #[error("Failure")]
    Failure(StatusCode),

    #[error("Invalid MIME Type")]
    InvalidMimeType,

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
}

async fn inner(client: Arc<Client>, url: String) -> Result<(Timestamp, Embed), Error> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(Error::InvalidUrl);
    }

    let mut resp = client.get(url.as_str()).send().await?;

    if !resp.status().is_success() {
        return Err(Error::Failure(resp.status()));
    }

    let link = resp
        .headers()
        .get("link")
        .and_then(|h| h.to_str().ok())
        .map(|h| embed_parser::oembed::parse_link_header(h));

    let mut embed = sdk::models::EmbedV1::default();
    let mut oembed: Option<OEmbed> = None;
    let mut max_age = 0;

    embed.url = Some(url.as_str().into());

    if let Some(json_link) = link
        .as_ref()
        .and_then(|l| l.iter().find(|o| o.format == OEmbedFormat::JSON))
    {
        if let Ok(o) = fetch_oembed(&client, json_link).await {
            oembed = Some(o);
        }
    }

    drop(link);

    if let Some(mime) = resp.headers().get("content-type").and_then(|h| h.to_str().ok()) {
        let Some(mime) = mime.split(';').next() else {
            return Err(Error::InvalidMimeType);
        };

        if mime == "text/html" {
            let mut html = Vec::with_capacity(512);
            if let Some(mut headers) = read_head(&mut resp, &mut html).await? {
                headers.sort_by_key(|meta| match meta {
                    Header::Meta(meta) => meta.property,
                    Header::Link(link) => link.href,
                });

                let extra = embed::parse_meta_to_embed(&mut embed, &headers);

                match extra.link {
                    Some(link) if oembed.is_none() && link.format == OEmbedFormat::JSON => {
                        if let Ok(o) = fetch_oembed(&client, &link).await {
                            oembed = Some(o);
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
                            media.w = Some(image_size.width as _);
                            media.h = Some(image_size.height as _);
                        }
                    }

                    embed.ty = EmbedType::Img;
                    embed.img = Some(media);
                }
                Some("video") => {
                    embed.ty = EmbedType::Vid;
                    embed.vid = Some(media);
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

    // naively resolve relative paths
    {
        // https: / / whatever.com /
        let root_idx = url.split('/').map(|s| s.len()).take(3).sum::<usize>();
        let (https, root) = {
            let mut root = url[..(root_idx + 2)].to_owned();
            root += "/";
            (root.starts_with("https://"), root)
        };
        embed.visit_media_mut(|media| {
            if media.url.starts_with("https://") || media.url.starts_with("http://") {
                return;
            }

            if media.url.starts_with(".") {
                // TODO
            }

            let old = media.url.as_str();

            media.url = 'media_url: {
                let mut url = root.clone();

                // I've seen this before, where "https://" is replaced with "undefined//"
                if old.starts_with("undefined//") {
                    url = if https { "https://" } else { "http://" }.to_owned();
                    url += &old["undefined//".len()..];
                    break 'media_url url.into();
                }

                url += &old;
                url.into()
            };
        });
    }

    // after relative paths are resolved, try to find image dimensions
    resolve_images(&client, &mut embed).await?;

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

async fn fetch_oembed<'a>(client: &reqwest::Client, link: &OEmbedLink<'a>) -> Result<OEmbed, Error> {
    client
        .get(link.url)
        .send()
        .await?
        .json::<OEmbed>()
        .await
        .map_err(Error::from)
}

async fn read_head<'a>(
    resp: &'a mut reqwest::Response,
    html: &'a mut Vec<u8>,
) -> Result<Option<embed_parser::html::HeaderList<'a>>, Error> {
    while let Some(chunk) = resp.chunk().await? {
        html.extend(&chunk);

        if memchr::memmem::rfind(&html, b"</body").is_some() {
            break;
        }

        // 1MB of HTML downloaded, assume it's a broken page or DoS attack and don't bother with more
        if html.len() > (1024 * 1024) {
            return Ok(None);
        }
    }

    Ok(match std::str::from_utf8(html) {
        Ok(html) => embed_parser::html::parse_meta(html),
        Err(_) => None,
    })
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

// TODO: Fetch these in parallel?
async fn resolve_images(client: &reqwest::Client, embed: &mut EmbedV1) -> Result<(), Error> {
    if let Some(ref mut media) = embed.img {
        let _ = resolve_media(client, &mut *media).await;
    }

    if let Some(ref mut media) = embed.thumb {
        let _ = resolve_media(client, &mut *media).await;
    }

    if let Some(ref mut footer) = embed.footer {
        if let Some(ref mut media) = footer.icon {
            let _ = resolve_media(client, &mut *media).await;
        }
    }

    if let Some(ref mut author) = embed.author {
        if let Some(ref mut media) = author.icon {
            let _ = resolve_media(client, &mut *media).await;
        }
    }

    Ok(())
}

async fn resolve_media(client: &reqwest::Client, media: &mut EmbedMedia) -> Result<(), Error> {
    // already has dimensions
    if !matches!((media.w, media.h), (None, None)) {
        return Ok(());
    }

    // TODO: Remove when relative paths are handled
    if media.url.starts_with(".") {
        return Ok(());
    }

    let mut resp = client.get(&*media.url).send().await?;

    if let Some(mime) = resp.headers().get("content-type").and_then(|h| h.to_str().ok()) {
        media.mime = Some(mime.into());

        if mime.starts_with("image") {
            let mut bytes = Vec::with_capacity(512);

            if let Ok(_) = read_bytes(&mut resp, &mut bytes, 1024 * 512).await {
                if let Ok(image_size) = imagesize::blob_size(&bytes) {
                    media.w = Some(image_size.width as _);
                    media.h = Some(image_size.height as _);
                }
            }
        }
    }

    Ok(())
}
