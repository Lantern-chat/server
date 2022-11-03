use embed_parser::{embed::parse_meta_to_embed, html::Header};
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
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    log_request(&req);

    // Optionally, get more helpful error messages written to the console in the case of a panic.
    utils::set_panic_hook();

    if let Ok(Some(url)) = req.headers().get("") {
        let mut resp = Fetch::Request(Request::new(&url, Method::Get)?).send().await?;

        if resp.status_code() != 200 {
            return Response::error("", resp.status_code());
        }

        let link_header = resp.headers().get("link")?;

        let link = link_header
            .as_ref()
            .map(|h| embed_parser::oembed::parse_link_header(&h));

        if let Some(mime) = resp.headers().get("content-type")? {
            if mime == "text/html" {
                let mut html = Vec::with_capacity(512);
                if let Some(mut metas) = read_head(&mut resp, &mut html).await? {
                    metas.sort_by_key(|meta| match meta {
                        Header::Meta(meta) => meta.property,
                        Header::Link(link) => link.href,
                    });

                    //let mut embed = sdk::models::Embed::default();
                    //parse_meta_to_embed(&mut embed, &metas);
                    // do stuff
                }

                drop(html); // ensure it lives long enough
            } else if matches!(mime.get(0..6), Some("image" | "video" | "audio")) {
            }
        }
    }

    Response::error("", 404)
}

async fn read_head<'a>(
    resp: &'a mut Response,
    html: &'a mut Vec<u8>,
) -> Result<Option<embed_parser::html::HeaderList<'a>>> {
    use futures_util::StreamExt;

    let mut stream = resp.stream()?;

    while let Some(chunk) = stream.next().await {
        html.extend(chunk?);

        if memchr::memmem::rfind(&html, b"</head").is_some() {
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
