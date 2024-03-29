#![allow(unused_imports)]

pub extern crate tokio_postgres as pg;

extern crate tracing as log;

pub mod migrate;
pub mod pool;

pub use pg::Row;

#[derive(Clone)]
pub struct DatabasePools {
    pub read: pool::Pool,
    pub write: pool::Pool,
}
