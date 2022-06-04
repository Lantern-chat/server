use embed_parser::html::HeaderList;
use headers::{ContentType, HeaderMapExt};

use crate::Error;
use crate::State;

pub struct OEmbedClient {
    client: reqwest::Client,
}

impl OEmbedClient {
    pub fn new() -> Result<OEmbedClient, Error> {
        Ok(OEmbedClient {
            client: super::create_service_client()?,
        })
    }

    /// Fetch a webpage and parse any headers or meta tags,
    /// then also fetch associated oembed information for that URL
    pub async fn fetch(&self, state: State, url: &str) -> Result<(), Error> {
        let req = self.client.get(url).build()?;
        let mut resp = self.client.execute(req).await?;

        if !resp.status().is_success() {
            return Ok(());
        }

        let headers = resp.headers();

        let mut links = smallvec::SmallVec::<[_; 2]>::new();
        for link in headers.get_all("link") {
            if let Ok(s) = link.to_str() {
                links.extend(embed_parser::oembed::parse_link_header(s));
            }
        }
        drop(links); // TODO: Process links instead

        if let Some(ct) = headers.typed_get::<ContentType>() {
            let m = mime::Mime::from(ct);

            match (m.type_(), m.subtype()) {
                (mime::TEXT, mime::HTML) => {
                    let mut html = Vec::with_capacity(512);
                    if let Some(metas) = read_head(&mut resp, &mut html).await? {
                        // do stuff
                    }

                    drop(html); // ensure it lives long enough
                }
                (mime::IMAGE | mime::VIDEO | mime::AUDIO, _) => { /* TODO */ }
                _ => {}
            }
        }

        Ok(())
    }

    /// Fetch oEmbed data from an oEmbed provider
    pub async fn fetch_oembed(&self, state: State, url: &str) -> Result<(), Error> {
        unimplemented!()
    }
}

async fn read_head<'a>(
    resp: &mut reqwest::Response,
    html: &'a mut Vec<u8>,
) -> Result<Option<HeaderList<'a>>, Error> {
    while let Some(chunk) = resp.chunk().await? {
        html.extend(chunk);

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
