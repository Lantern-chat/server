#[cfg(feature = "rust-argon2")]
pub mod v1;

#[cfg(feature = "rustcrypto-argon2")]
pub mod v2;

pub use v2::*;

const OUTPUT_LEN: usize = 32;
const MEM_COST: u32 = 12 * 1024;
const PARALLELISM: u32 = 1;
const TIME_COST: u32 = 3;
