use tokio::sync::Semaphore;

use crate::ctrl::Error;

pub struct OEmbedClient {
    client: reqwest::Client,
    limit: Semaphore,
}

impl OEmbedClient {
    pub fn new() -> Result<OEmbedClient, Error> {
        Ok(OEmbedClient {
            client: super::create_service_client()?,
            limit: Semaphore::new(num_cpus::get() * 16),
        })
    }

    /// Fetch a webpage and parse any headers or meta tags,
    /// then also fetch associated oembed information for that URL
    pub async fn fetch_all(&self, url: &str) -> Result<(), Error> {
        let _guard = self.limit.acquire().await?;

        unimplemented!()
    }

    /// Fetch oEmbed data from an oEmbed provider
    pub async fn fetch_oembed(&self, url: &str) -> Result<(), Error> {
        let _guard = self.limit.acquire().await?;

        unimplemented!()
    }
}
