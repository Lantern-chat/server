pub mod auth;
pub mod encoding;
pub mod file_cache;
pub mod rate_limit;
pub mod routes;
pub mod service;

use std::sync::LazyLock;

pub static METHOD_QUERY: LazyLock<http::Method> = LazyLock::new(|| http::Method::from_bytes(b"QUERY").unwrap());
