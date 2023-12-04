use std::{ops::Deref, sync::Arc};

use crate::{
    config::Config,
    web::{file_cache::MainFileCache, rate_limit::RateLimitTable},
};

use arc_swap::ArcSwap;
use futures::{Stream, StreamExt};
use sdk::Snowflake;
use tokio::sync::{Notify, Semaphore};

pub mod session_cache;

pub struct InnerServerState {
    pub db: db::DatabasePools,
    pub config: config::Config<Config>,

    pub session_cache: session_cache::AuthCache,
    pub rate_limit: RateLimitTable,
    pub file_cache: MainFileCache,
    pub hasher: ahash::RandomState,
    pub emoji: common::emoji::EmojiMap,
}

#[derive(Clone)]
#[repr(transparent)]
pub struct ServerState(triomphe::Arc<InnerServerState>);

impl Deref for ServerState {
    type Target = InnerServerState;

    #[inline(always)]
    fn deref(&self) -> &InnerServerState {
        &self.0
    }
}

impl config::HasConfig<Config> for ServerState {
    #[inline(always)]
    fn raw(&self) -> &config::Config<Config> {
        &self.config
    }
}
