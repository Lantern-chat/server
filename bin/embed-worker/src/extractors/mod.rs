use crate::{Error, Params, WorkerState};

pub type EmbedWithExpire = (iso8601_timestamp::Timestamp, sdk::models::Embed);

#[async_trait::async_trait]
pub trait Extractor {
    fn matches(&self, domain: &str) -> bool;

    async fn extract(
        &self,
        state: std::sync::Arc<WorkerState>,
        url: url::Url,
        params: Params,
    ) -> Result<EmbedWithExpire, Error>;
}

pub mod generic;
