#![cfg_attr(not(debug_assertions), allow(unused_mut, unused_variables, unused_imports))]
#![allow(clippy::redundant_pattern_matching, clippy::identity_op, clippy::redundant_closure)]
#![deny(deprecated)]

#[macro_use]
extern crate serde;

extern crate tracing as log;

pub mod auth;
pub mod config;
pub mod error;
pub mod state;
pub mod web;

pub mod prelude {
    pub use crate::config::Config;
    pub use crate::error::Error;
    pub use crate::state::ServerState;
    pub use rpc::auth::Authorization;

    pub use config::HasConfig;
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

    Ok(())
}
