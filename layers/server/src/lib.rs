#![allow(unused_imports, clippy::redundant_pattern_matching)]
#![deny(deprecated)]

#[macro_use]
extern crate serde;

#[macro_use]
extern crate async_recursion;

extern crate tracing as log;

pub extern crate config;

pub mod built {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub mod backend;
pub mod error;
//pub mod net;
pub mod metrics;
pub mod state;
pub mod tasks;
pub mod util;
pub mod web;

pub(crate) use backend::Authorization;
pub use error::Error;
pub use state::ServerState;
