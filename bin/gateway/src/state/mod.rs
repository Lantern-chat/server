use std::ops::Deref;
use triomphe::Arc;

use crate::{
    config::Config,
    gateway::Gateway,
    //web::{file_cache::MainFileCache, rate_limit::RateLimitTable},
    web::file_cache::StaticFileCache,
};

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
    pub rpc: ::rpc::client::RpcManager,
    pub gateway: Gateway,
}

#[derive(Clone)]
#[repr(transparent)]
pub struct GatewayServerState(triomphe::Arc<InnerServerState>);

impl Deref for GatewayServerState {
    type Target = InnerServerState;

    #[inline(always)]
    fn deref(&self) -> &InnerServerState {
        &self.0
    }
}

impl config::HasConfig<Config> for GatewayServerState {
    #[inline(always)]
    fn raw(&self) -> &config::Config<Config> {
        &self.config
    }
}

impl GatewayServerState {
    pub fn new(config: crate::config::Config, nexus: ::rpc::client::RpcClient) -> Self {
        Self(Arc::new(InnerServerState {
            sf: SnowflakeGenerator::new(sdk::models::sf::LANTERN_EPOCH, 0),
            config: config::Config::new(config),
            auth_cache: auth_cache::AuthCache::default(),
            file_cache: StaticFileCache::default(),
            emoji: common::emoji::EmojiMap::default(),
            rpc: ::rpc::client::RpcManager::new(nexus),
            gateway: Gateway::default(),
        }))
    }
}
