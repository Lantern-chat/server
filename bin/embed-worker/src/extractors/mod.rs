#![allow(unused)]

use crate::{
    config::{Config, ConfigError},
    Error, Params, WorkerState,
};
use std::sync::Arc;

use url::Url;

pub type EmbedWithExpire = (iso8601_timestamp::Timestamp, sdk::models::Embed);

pub trait ExtractorFactory {
    fn create(&self, config: &Config) -> Result<Option<Box<dyn Extractor>>, ConfigError>;
}

#[async_trait::async_trait]
pub trait Extractor: Send + Sync + std::fmt::Debug {
    /// Test if this extractor should be used for this domain
    fn matches(&self, url: &Url) -> bool;

    /// Optional setup stage for extractor initialization on program start (i.e. login to services)
    async fn setup(&self, _state: Arc<WorkerState>) -> Result<(), Error> {
        Ok(())
    }

    async fn extract(&self, state: Arc<WorkerState>, url: Url, params: Params) -> Result<EmbedWithExpire, Error>;
}

pub mod generic;

pub mod wikipedia;
#[rustfmt::skip]
pub fn extractor_factories() -> Vec<Box<dyn ExtractorFactory>> {
    vec![
        Box::new(wikipedia::WikipediaExtractorFactory),
        Box::new(generic::GenericExtractor),
    ]
}
