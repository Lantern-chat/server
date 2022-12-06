use std::time::Duration;

use pg::Config as PgConfig;

#[derive(Clone, Debug)]
pub struct Timeouts {
    /// Timeout when waiting for a slot to become available
    pub wait: Option<Duration>,
    /// Timeout when creating a new object
    pub create: Option<Duration>,
    /// Timeout when recycling an object
    pub recycle: Option<Duration>,
}

impl Timeouts {
    pub fn wait(mut self, timeout: Duration) -> Self {
        self.wait = Some(timeout);
        self
    }

    pub fn create(mut self, timeout: Duration) -> Self {
        self.create = Some(timeout);
        self
    }

    pub fn recycle(mut self, timeout: Duration) -> Self {
        self.recycle = Some(timeout);
        self
    }
}

impl Default for Timeouts {
    /// Create a timeout config with no timeouts set
    fn default() -> Self {
        Self {
            create: None,
            wait: None,
            recycle: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecyclingMethod {
    Fast,
    Verified,
    Clean,
}

impl RecyclingMethod {
    pub fn query(self) -> Option<&'static str> {
        match self {
            RecyclingMethod::Fast => None,
            RecyclingMethod::Verified => Some(""),
            RecyclingMethod::Clean => Some({
                "CLOSE ALL;
                SET SESSION AUTHORIZATION DEFAULT;
                RESET ALL;
                UNLISTEN *;
                SELECT pg_advisory_unlock_all();
                DISCARD TEMP;
                DISCARD SEQUENCES;"
            }),
        }
    }
}

impl Default for RecyclingMethod {
    fn default() -> Self {
        RecyclingMethod::Fast
    }
}

#[derive(Clone, Debug, Default)]
pub struct PoolConfig {
    pub pg_config: PgConfig,
    pub timeouts: Timeouts,
    pub readonly: bool,
    pub max_connections: usize,
    pub max_retries: usize,
    pub channel_size: usize,
    pub recycling_method: RecyclingMethod,
}

impl PoolConfig {
    pub fn new(pg_config: PgConfig) -> Self {
        PoolConfig {
            pg_config,
            timeouts: Timeouts::default(),
            readonly: false,
            max_connections: num_cpus::get_physical() * 4,
            max_retries: 6,
            channel_size: 64,
            recycling_method: RecyclingMethod::Fast,
        }
    }

    pub fn readonly(mut self) -> Self {
        self.readonly = true;
        self
    }

    pub fn max_connections(mut self, size: usize) -> Self {
        self.max_connections = size;
        self
    }

    pub fn channel_size(mut self, size: usize) -> Self {
        self.channel_size = size;
        self
    }

    pub fn max_retries(mut self, retries: usize) -> Self {
        self.max_retries = retries;
        self
    }
}

impl std::str::FromStr for PoolConfig {
    type Err = <PgConfig as std::str::FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(PoolConfig::new)
    }
}
