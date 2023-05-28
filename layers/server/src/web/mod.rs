pub mod auth;
pub mod encoding;
pub mod file_cache;
pub mod gateway;
pub mod rate_limit;
pub mod response;
pub mod routes;
pub mod service;

lazy_static::lazy_static! {
    pub static ref METHOD_QUERY: http::Method = http::Method::from_bytes(b"QUERY").unwrap();
}
