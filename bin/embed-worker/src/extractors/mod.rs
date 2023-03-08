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

mod prelude {
    pub use std::fmt::Write;
    pub use std::sync::Arc;

    pub use embed_parser::oembed::{OEmbed, OEmbedFormat, OEmbedLink};
    pub use futures_util::FutureExt;
    pub use reqwest::{
        header::{HeaderName, HeaderValue},
        Method, StatusCode,
    };
    pub use smol_str::{SmolStr, ToSmolStr};
    pub use url::Url;

    pub use sdk::models::embed::v1::*;

    pub(crate) use crate::{
        config::{Config, ConfigError},
        Error, Params, Site, WorkerState,
    };

    pub use super::{generic, EmbedWithExpire, Extractor, ExtractorFactory};
}

pub mod generic;

pub mod deviantart;
pub mod e621;
pub mod furaffinity;
pub mod imgur;
pub mod inkbunny;
pub mod wikipedia;

#[rustfmt::skip]
pub fn extractor_factories() -> Vec<Box<dyn ExtractorFactory>> {
    vec![
        Box::new(e621::E621ExtractorFactory),
        Box::new(wikipedia::WikipediaExtractorFactory),
        Box::new(deviantart::DeviantArtExtractor),
        Box::new(imgur::ImgurExtractorFactory),
        Box::new(inkbunny::InkbunnyExtractorFactory),
        Box::new(furaffinity::FurAffinityExtractorFactory),
        Box::new(generic::GenericExtractor),
    ]
}
