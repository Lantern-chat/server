#[macro_use]
extern crate serde;

extern crate tracing as log;

pub extern crate config;

pub mod built {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub mod error;
//pub mod net;
pub mod state;
pub mod util;
pub mod web;

pub mod tasks;

pub use state::ServerState;
