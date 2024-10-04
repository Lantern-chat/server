use std::ops::Deref;
use triomphe::Arc;

use crate::{
    config::Config,
    gateway::{Gateway, Heart},
    //web::{file_cache::MainFileCache, rate_limit::RateLimitTable},
    web::file_cache::StaticFileCache,
};

use arc_swap::ArcSwap;
use futures::{Stream, StreamExt};
use tokio::sync::{Notify, Semaphore};

use schema::sf::SnowflakeGenerator;

pub mod auth_cache;

pub struct InnerServerState {
    pub sf: SnowflakeGenerator,
    pub config: config::Config<Config>,
    pub auth_cache: auth_cache::AuthCache,
    pub file_cache: StaticFileCache,
    pub emoji: common::emoji::EmojiMap,
    pub rpc: rpc::client::RpcManager,
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
