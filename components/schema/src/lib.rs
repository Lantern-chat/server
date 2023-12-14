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
pub mod email;
pub mod flags;
pub mod names;
pub mod roles;
pub mod search;
pub mod validation;

pub fn has_all_permission_bits(
    perms: sdk::models::Permissions,
    cols: (impl thorn::ValueExpr, impl thorn::ValueExpr),
) -> impl thorn::BooleanExpr {
    use thorn::*;

    let perms = perms.to_i64();
    cols.0.has_all_bits(perms[0].lit()).and(cols.1.has_all_bits(perms[1].lit()))
}

/// Wrapper around [`thorn::sql!`] which injects `use schema::*` ahead of the declaration
#[macro_export]
macro_rules! sql {
    ($($tt:tt)*) => {{ #![allow(unused_imports)] $crate::thorn::sql! { use $crate::*; $($tt)* } }};
}
