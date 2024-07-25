extern crate tracing as log;

pub use pg_pool::*;

#[derive(Clone)]
pub struct DatabasePools {
    pub read: Pool,
    pub write: Pool,
}

pub mod migrate;
