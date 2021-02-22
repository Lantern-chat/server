//! FTL is the internal web framework derived from parts of warp,
//! but designed for a more imperative workflow.

pub mod body;
pub mod fs;
pub mod rate_limit;
pub mod real_ip;
pub mod reply;
pub mod route;
pub mod ws;
pub mod multipart;

pub use http::StatusCode;

pub use self::reply::{Reply, Response};
pub use self::route::{
    BodyError, Route,
    Segment::{self, End, Exact},
};
