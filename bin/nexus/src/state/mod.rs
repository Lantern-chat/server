use schema::sf::SnowflakeGenerator;
use tokio::sync::Semaphore;

use crate::prelude::*;
use crate::{config::Config, gateway::Gateway, queues::Queues, services::Services};

pub mod permission_cache;
use permission_cache::PermissionCache;

use common::{emoji::EmojiMap, id_lock::IdLockMap};

pub struct ServerStateInner {
    pub sf: SnowflakeGenerator,

    pub db: db::DatabasePools,
    pub config: config::Config<Config>,

    /// Generic lock for anything with a Snowflake ID
    pub id_lock: IdLockMap,

    /// Each permit represents 1 Kibibyte
    ///
    /// Used to limit how many memory-intensive tasks are run at a time
    pub mem_semaphore: Semaphore,

    /// Used to limit how many CPU-intensive tasks are run at a time
    pub cpu_semaphore: Semaphore,

    pub perm_cache: PermissionCache,

    pub gateway: Gateway,

    pub services: Services,
    pub queues: Queues,

    // /// Generic lock for anything with a Snowflake ID
    // pub id_lock: id_lock::IdLockMap,
    /// Hasher for general use
    pub hasher: sdk::FxRandomState2,

    pub emoji: EmojiMap,

    /// Last timestep used for MFA per-user.
    pub mfa_last: scc::HashIndex<UserId, u64, sdk::FxRandomState2>,
}

#[derive(Clone)]
#[repr(transparent)]
pub struct ServerState(triomphe::Arc<ServerStateInner>);

impl std::ops::Deref for ServerState {
    type Target = ServerStateInner;

    #[inline(always)]
    fn deref(&self) -> &ServerStateInner {
        &self.0
    }
}

impl config::HasConfig<Config> for ServerState {
    #[inline(always)]
    fn raw(&self) -> &config::Config<Config> {
        &self.config
    }
}

impl ServerState {
    pub fn new(config: Config, db: db::DatabasePools) -> Self {
        ServerState(triomphe::Arc::new(ServerStateInner {
            db,

            id_lock: Default::default(),
            mem_semaphore: Semaphore::new(config.local.general.memory_limit as usize),
            cpu_semaphore: Semaphore::new(config.local.general.cpu_limit as usize),
            perm_cache: PermissionCache::default(),
            emoji: Default::default(),
            hasher: sdk::FxRandomState2::default(),

            mfa_last: Default::default(),

            sf: SnowflakeGenerator::new(sdk::models::sf::LANTERN_EPOCH, 0),

            gateway: Gateway::default(),
            services: Services::start().expect("Failed to start services"),
            queues: Queues::default(),

            config: config::Config::new(config),
        }))
    }
}
