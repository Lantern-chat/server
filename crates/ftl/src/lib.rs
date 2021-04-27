//! FTL is the internal web framework derived from parts of warp,
//! but designed for a more imperative workflow.

#![allow(unused_imports)]

extern crate tracing as log;

pub mod body;
pub mod compression;
pub mod fs;
pub mod multipart;
pub mod real_ip;
pub mod reply;
pub mod route;
pub mod ws;

pub use http::{Method, StatusCode};

pub use self::reply::{Reply, ReplyError, Response};
pub use self::route::{
    BodyError, Route,
    Segment::{self, End, Exact},
};
