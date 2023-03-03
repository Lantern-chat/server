use crate::{Error, Params, WorkerState};
use std::sync::Arc;

pub type EmbedWithExpire = (iso8601_timestamp::Timestamp, sdk::models::Embed);

#[async_trait::async_trait]
pub trait Extractor {
    /// Test if this extractor should be used for this domain
    fn matches(&self, domain: &str) -> bool;

    /// Optional setup stage for extractor initialization on program start (i.e. login to services)
    async fn setup(&self, _state: Arc<WorkerState>) -> Result<(), Error> {
        Ok(())
    }

    async fn extract(
        &self,
        state: Arc<WorkerState>,
        url: url::Url,
        params: Params,
    ) -> Result<EmbedWithExpire, Error>;
}

pub mod generic;
