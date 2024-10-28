#![cfg_attr(not(debug_assertions), allow(unused_mut, unused_variables, unused_imports))]
#![allow(clippy::redundant_pattern_matching, clippy::identity_op, clippy::redundant_closure)]
#![deny(deprecated)]

extern crate tracing as log;

pub mod state;

pub mod prelude {
    pub use crate::state::CdnServerState;
    pub use common_web::error::Error;

    pub use rpc::{auth::Authorization, event::ServerEvent};
    pub use sdk::models::{aliases::*, Nullable, SmolStr, Snowflake, Timestamp};

    pub type EventId = sdk::Snowflake;
    pub type ConnectionId = sdk::Snowflake;

    //pub use crate::config::Config; // TODO
    pub use config::HasConfig;

    pub use rkyv::Archived;
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // NOTE: tokio_uring uses the Tokio current_thread runtime internally
    tokio_uring::start(run())
}

pub mod built {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}
