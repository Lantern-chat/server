#![cfg_attr(not(debug_assertions), allow(unused_mut, unused_variables, unused_imports))]
#![allow(clippy::redundant_pattern_matching, clippy::identity_op, clippy::redundant_closure)]
#![deny(deprecated)]

#[macro_use]
extern crate serde;

extern crate tracing as log;

pub mod allocator;
pub mod auth;
pub mod cli;
pub mod config;
pub mod error;
pub mod rpc;
pub mod state;
pub mod web;

pub mod prelude {
    pub use crate::error::Error;
    pub use crate::state::ServerState;

    pub use rpc::{auth::Authorization, event::ServerEvent, simple_de};
    pub use sdk::models::{Nullable, SmolStr, Snowflake, Timestamp};

    pub use crate::config::Config;
    pub use config::HasConfig;

    pub use rkyv::Archived;
}

pub mod built {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    println!("Build time: {}", built::BUILT_TIME_UTC);

    dotenv::dotenv()?;

    let mut config = config::LocalConfig::default();
    ::config::Configuration::configure(&mut config);

    println!("Hello, world!");

    #[inline(never)]
    fn get_any<T>() -> T {
        unimplemented!()
    }

    _ = auth::do_auth(get_any(), get_any()).await;

    Ok(())
}
