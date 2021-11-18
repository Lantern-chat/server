use std::time::Duration;

use crate::html::parse_meta;

pub struct OEmbedClient {
    client: reqwest::Client,
}

#[derive(Debug, thiserror::Error)]
pub enum OEmbedError {
    #[error("Network {0}")]
    Network(#[from] reqwest::Error),
}

impl OEmbedClient {
    pub fn new() -> Result<Self, OEmbedError> {
        let client = reqwest::ClientBuilder::new()
            .user_agent("Mozzila/5.0 (compatible; Lantern Chat)")
            .gzip(true)
            .deflate(true)
            .brotli(true)
            .redirect(reqwest::redirect::Policy::limited(1))
            .connect_timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(false)
            .build()?;

        Ok(OEmbedClient { client })
    }

    pub async fn process(&self, webpage: &str) -> Result<Option<()>, OEmbedError> {
        let res = self.client.get(webpage).send().await?;

        if !res.status().is_success() {
            return Ok(None);
        }

        let html = res.text().await?;

        parse_meta(&html);

        unimplemented!()
    }
}
