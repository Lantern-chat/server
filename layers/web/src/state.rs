use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicBool, AtomicI64, Ordering},
        Arc,
    },
};

use config::Config;

use crate::web::{file_cache::MainFileCache, rate_limit::RateLimitTable};

pub struct InnerServerState {
    pub backend: backend::State,
    pub rate_limit: RateLimitTable,
    pub file_cache: MainFileCache,
}

#[derive(Clone)]
pub struct ServerState(Arc<InnerServerState>);

impl Deref for ServerState {
    type Target = InnerServerState;

    fn deref(&self) -> &InnerServerState {
        &*self.0
    }
}

impl ServerState {
    pub fn new(backend: backend::State) -> Self {
        ServerState(Arc::new(InnerServerState {
            backend,
            rate_limit: RateLimitTable::new(),
            file_cache: MainFileCache::default(),
        }))
    }

    pub fn config(&self) -> &Config {
        &self.backend.config
    }
}
