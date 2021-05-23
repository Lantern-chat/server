use tokio_postgres::Config as PgConfig;

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
    pub readonly: bool,
    pub max_connections: usize,
    pub channel_size: usize,
    pub recycling_method: RecyclingMethod,
}

impl PoolConfig {
    pub fn new(pg_config: PgConfig) -> Self {
        PoolConfig {
            pg_config,
            readonly: false,
            max_connections: num_cpus::get_physical() * 4,
            channel_size: 256,
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
}
