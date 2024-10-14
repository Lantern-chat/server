#![cfg_attr(not(debug_assertions), allow(unused_mut, unused_variables, unused_imports))]
#![allow(clippy::redundant_pattern_matching, clippy::identity_op, clippy::redundant_closure)]
#![deny(deprecated)]

#[macro_use]
extern crate serde;

extern crate tracing as log;

pub mod allocator;
pub mod cli;
pub mod config;
pub mod gateway;
pub mod rpc;
pub mod state;
pub mod tasks;
pub mod web;

pub mod prelude {
    pub use crate::state::GatewayServerState;
    pub use common_web::error::Error;

    pub use rpc::{auth::Authorization, event::ServerEvent};
    pub use sdk::models::{aliases::*, Nullable, SmolStr, Snowflake, Timestamp};

    pub type EventId = sdk::Snowflake;
    pub type ConnectionId = sdk::Snowflake;

    pub use crate::config::Config;
    pub use config::HasConfig;

    pub use rkyv::Archived;
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    println!("Build time: {}", built::BUILT_TIME_UTC);

    dotenv::dotenv()?;

    let mut config = config::LocalConfig::default();
    ::config::Configuration::configure(&mut config);

    Ok(())
}

pub mod built {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}
