extern crate client_sdk as sdk;

use embed_parser::{
    embed,
    html::Header,
    oembed::{OEmbed, OEmbedFormat, OEmbedLink},
};
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

#[event(fetch)]
pub async fn main(mut req: Request, _env: Env, _ctx: worker::Context) -> Result<Response> {
    if req.method() != Method::Post {
        return Response::error("Method Not Allowed", 405);
    }

    log_request(&req);

    #[cfg(debug_assertions)]
    utils::set_panic_hook();

    let url = req.text().await?;

    if url.starts_with("https://") || url.starts_with("http://") {
        fetch_source(url).await
    } else {
        Response::error("Invalid URL", 400)
    }
}

async fn fetch_source(url: String) -> Result<Response> {
    let mut resp = Fetch::Request(Request::new_with_init(&url, &req_init(Method::Get)?)?)
        .send()
        .await?;

    if resp.status_code() != 200 {
        return Response::error(resp.text().await?, resp.status_code());
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
        if let Ok(o) = fetch_oembed(json_link).await {
            oembed = Some(o);
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
                        if let Ok(o) = fetch_oembed(&link).await {
                            oembed = Some(o);
                        }
                    }
                    _ => {}
                }

                max_age = extra.max_age;
            }

            drop(html); // ensure it lives long enough
        } else if matches!(mime.get(0..6), Some("image" | "video" | "audio")) {
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

    let expires = {
        use iso8601_timestamp::{Timestamp, Duration};

        embed.ts = Timestamp::now_utc();

        // limit max_age to 1 month
        embed.ts.checked_add(Duration::seconds(max_age.min(60 * 60 * 24 * 30) as i64))
    };

    Response::from_json(&(expires, embed))
}

async fn fetch_oembed<'a>(link: &OEmbedLink<'a>) -> Result<OEmbed> {
    Fetch::Request(Request::new_with_init(&link.url, &req_init(Method::Get)?)?)
        .send()
        .await?
        .json::<OEmbed>()
        .await
}

fn req_init(method: Method) -> Result<RequestInit> {
    Ok(RequestInit {
        body: None,
        method,
        headers: {
            let mut headers = Headers::new();
            headers.append(
                "user-agent",
                "Mozilla/5.0 (compatible; Lantern Embed; +https://lantern.chat)",
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
