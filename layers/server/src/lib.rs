#![cfg_attr(not(debug_assertions), allow(unused_mut, unused_variables, unused_imports))]
#![allow(clippy::redundant_pattern_matching, clippy::identity_op, clippy::redundant_closure)]
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
