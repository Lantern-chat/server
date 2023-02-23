use crate::{Error, ServerState};

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

        let res = self.client.post(uri).body(url).send().await?;

        // embed worker may not succeed, and that's okay, just don't return an embed
        if !res.status().is_success() {
            return Ok(None);
        }

        res.json().await.map_err(Error::from)
    }
}
