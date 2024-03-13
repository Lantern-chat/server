use crate::prelude::*;

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
        use rand::seq::SliceRandom;
        use reqwest::header::{HeaderName, HeaderValue};
        use std::borrow::Cow;

        let config = state.config_full();

        let Some(uri) = config.shared.embed_worker_uris.choose(&mut rand::thread_rng()) else {
            log::warn!("No Embed Worker URIs configured!");

            return Ok(None);
        };

        let uri = match language {
            Some(l) => Cow::Owned(format!("{uri}?l={l}")),
            None => Cow::Borrowed(uri.as_str()),
        };

        let resp = self
            .client
            .post(uri.as_ref())
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
