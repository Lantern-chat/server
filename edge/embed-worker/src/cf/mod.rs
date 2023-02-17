use embed_parser::{
    embed,
    html::Header,
    oembed::{OEmbed, OEmbedFormat, OEmbedLink},
};
use sdk::models::*;
use worker::*;

mod utils;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or_else(|| "unknown region".into())
    );
}

use hmac::{digest::Key, Mac};
type Hmac = hmac::SimpleHmac<sha1::Sha1>;

use base64::engine::{general_purpose::URL_SAFE_NO_PAD, Engine};

#[event(fetch)]
pub async fn main(mut req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    if req.method() != Method::Post {
        return Response::error("Method Not Allowed", 405);
    }

    log_request(&req);

    #[cfg(debug_assertions)]
    utils::set_panic_hook();

    let url = req.text().await?;

    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Response::error("Invalid URL", 400);
    }

    let signing_key = {
        let hex_key = env.secret("CAMO_SIGNING_KEY")?.to_string();
        let mut raw_key = Key::<Hmac>::default();

        // keys are allowed to be shorter than the entire raw key. Will be padded internally.
        if let Err(_) = hex::decode_to_slice(&hex_key, &mut raw_key[..hex_key.len() / 2]) {
            return Response::error("", 500);
        }

        raw_key
    };

    let (https, root, domain) = embed_parser::utils::url_root(&url);

    let mut resp = Fetch::Request(Request::new_with_init(&url, &req_init(Method::Get, Some(domain))?)?)
        .send()
        .await?;

    if !(200..=299).contains(&resp.status_code()) {
        return Response::error("Failure", resp.status_code());
    }

    let link_header = resp.headers().get("link")?;

    let link = link_header
        .as_ref()
        .map(|h| embed_parser::oembed::parse_link_header(&h));

    let mut embed = sdk::models::EmbedV1::default();
    let mut oembed = None;
    let mut max_age = 0;

    embed.url = Some(url.as_str().into());

    if let Some(json_link) = link
        .as_ref()
        .and_then(|l| l.iter().find(|o| o.format == OEmbedFormat::JSON))
    {
        if let Ok(o) = fetch_oembed(json_link, domain).await {
            oembed = o;
        }
    }

    if let Some(mime) = resp.headers().get("content-type")? {
        let Some(mime) = mime.split(';').next() else {
            return Response::error("Invalid MIME Type", 400);
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
                        if let Ok(o) = fetch_oembed(&link, domain).await {
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

    embed_parser::quirks::resolve_relative(root, https, &mut embed);
    resolve_images(&mut embed).await?;
    embed_parser::quirks::fix_embed(&mut embed);

    embed.visit_media_mut(|media| {
        let sig = Hmac::new(&signing_key)
            .chain_update(&*media.url)
            .finalize()
            .into_bytes();

        let mut buf = [0; 27];
        if let Ok(27) = URL_SAFE_NO_PAD.encode_slice(sig, &mut buf) {
            use sdk::util::fixed::FixedStr;

            media.sig = Some(FixedStr::new(unsafe { std::str::from_utf8_unchecked(&buf) }));
        }
    });

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

    Response::from_json(&(expires, sdk::models::Embed::V1(embed)))
}

async fn fetch_oembed<'a>(link: &OEmbedLink<'a>, domain: &str) -> Result<Option<OEmbed>> {
    if embed_parser::quirks::AVOID_OEMBED.contains(domain) {
        return Ok(None);
    }

    Fetch::Request(Request::new_with_init(&link.url, &req_init(Method::Get, None)?)?)
        .send()
        .await?
        .json::<OEmbed>()
        .await
        .map(Some)
}

fn req_init(method: Method, domain: Option<&str>) -> Result<RequestInit> {
    Ok(RequestInit {
        body: None,
        method,
        headers: {
            let mut headers = Headers::new();
            headers.append(
                "user-agent",
                match domain.and_then(|d| embed_parser::quirks::USER_AGENTS.get(d)) {
                    Some(&user_agent) => user_agent,
                    None => "Lantern Embed Worker (bot; +https://github.com/Lantern-chat)",
                },
            )?;
            headers
        },
        ..RequestInit::default()
    })
}

async fn read_head<'a>(
    resp: &'a mut Response,
    html: &'a mut Vec<u8>,
) -> Result<Option<embed_parser::html::HeaderList<'a>>> {
    use futures_util::StreamExt;

    let mut stream = resp.stream()?;

    while let Some(chunk) = stream.next().await {
        html.extend(chunk?);

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

async fn read_bytes<'a>(resp: &'a mut Response, bytes: &'a mut Vec<u8>, max: usize) -> Result<()> {
    use futures_util::StreamExt;

    let mut stream = resp.stream()?;

    while let Some(chunk) = stream.next().await {
        bytes.extend(chunk?);

        if bytes.len() > max {
            break;
        }
    }

    Ok(())
}

// TODO: Fetch these in parallel?
async fn resolve_images(embed: &mut EmbedV1) -> Result<()> {
    if let Some(ref mut media) = embed.img {
        let _ = resolve_media(&mut *media).await;
    }

    if let Some(ref mut media) = embed.thumb {
        let _ = resolve_media(&mut *media).await;
    }

    if let Some(ref mut footer) = embed.footer {
        if let Some(ref mut media) = footer.icon {
            let _ = resolve_media(&mut *media).await;
        }
    }

    if let Some(ref mut author) = embed.author {
        if let Some(ref mut media) = author.icon {
            let _ = resolve_media(&mut *media).await;
        }
    }

    Ok(())
}

async fn resolve_media(media: &mut EmbedMedia) -> Result<()> {
    // already has dimensions
    if !matches!((media.w, media.h), (None, None)) {
        return Ok(());
    }

    // TODO: Remove when relative paths are handled
    if media.url.starts_with(".") {
        return Ok(());
    }

    let mut resp = Fetch::Request(Request::new_with_init(&media.url, &req_init(Method::Get, None)?)?)
        .send()
        .await?;

    if let Some(mime) = resp.headers().get("content-type")? {
        media.mime = Some(mime.as_str().into());

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
