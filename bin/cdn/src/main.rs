#![cfg_attr(not(debug_assertions), allow(unused_mut, unused_variables, unused_imports))]
#![allow(clippy::redundant_pattern_matching, clippy::identity_op, clippy::redundant_closure)]
#![deny(deprecated)]

extern crate tracing as log;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
}

pub mod built {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}
