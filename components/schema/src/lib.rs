#![cfg_attr(not(debug_assertions), allow(unused_mut, unused_variables, unused_imports))]
#![allow(clippy::single_char_add_str)]

#[macro_use]
extern crate serde;

pub extern crate auth;
pub extern crate db;
pub extern crate thorn;

pub mod codes;
pub use codes::*;

pub mod tables;
pub use tables::*;

pub mod views;
pub use views::*;

pub use thorn::pg::Type;

pub const SNOWFLAKE: Type = Type::INT8;
pub const SNOWFLAKE_ARRAY: Type = Type::INT8_ARRAY;

pub mod sf;
pub use sf::{Snowflake, SnowflakeExt};

pub mod asset;
pub mod config;
pub mod flags;
pub mod names;
pub mod roles;
pub mod search;
pub mod validation;

#[macro_export]
macro_rules! const_assert {
    ($($tt:tt)*) => { const _: () = assert!($($tt)*); };
}

/// Wrapper around [`thorn::sql!`] which injects `use schema::*` ahead of the declaration
#[macro_export]
macro_rules! sql {
    ($($tt:tt)*) => {{ #![allow(unused_imports)] $crate::thorn::sql! { use $crate::*; $($tt)* } }};
}
