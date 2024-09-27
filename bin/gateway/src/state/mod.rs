use std::ops::Deref;
use triomphe::Arc;

use crate::{
    config::Config,
    gateway::{Gateway, Heart},
    //web::{file_cache::MainFileCache, rate_limit::RateLimitTable},
    web::file_cache::MainFileCache,
};

use arc_swap::ArcSwap;
use futures::{Stream, StreamExt};
use tokio::sync::{Notify, Semaphore};

use schema::sf::SnowflakeGenerator;

pub mod session_cache;

pub struct InnerServerState {
    pub sf: SnowflakeGenerator,
    pub db: db::DatabasePools,
    pub config: config::Config<Config>,

    pub session_cache: session_cache::AuthCache,
    //pub rate_limit: RateLimitTable,
    pub file_cache: MainFileCache,
    pub hasher: sdk::FxRandomState2,
    pub emoji: common::emoji::EmojiMap,

    pub rpc: rpc::client::RpcManager,

    pub heart: Arc<Heart>,
    pub gateway: Gateway,
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
