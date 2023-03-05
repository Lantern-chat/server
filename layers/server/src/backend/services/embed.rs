use crate::{Error, ServerState};

use headers::{HeaderName, HeaderValue};
use sdk::models::{Embed, Timestamp};

pub struct EmbedClient {
    client: reqwest::Client,
}

impl EmbedClient {
    pub fn new() -> Result<EmbedClient, Error> {
        Ok(EmbedClient {
            client: super::create_service_client()?,
        })
    }

    pub async fn fetch(
        &self,
        state: &ServerState,
        url: String,
        language: Option<&str>,
    ) -> Result<Option<(Timestamp, Embed)>, Error> {
        let uri = &state.config().services.embed_worker_uri;
        let uri = match language {
            Some(l) => format!("{uri}?l={l}"),
            None => uri.clone(),
        };

        let resp = self
            .client
            .post(uri)
            .body(url)
            .header(
                HeaderName::from_static("content-type"),
                HeaderValue::from_static("text/plain; charset=utf-8"),
            )
            .send()
            .await?;

        // embed worker may not succeed, and that's okay, just don't return an embed
        if !resp.status().is_success() {
            return Ok(None);
        }

        resp.json().await.map_err(Error::from)
    }
}
