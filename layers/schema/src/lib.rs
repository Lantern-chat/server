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
pub mod flags;

pub mod auth;

pub fn has_all_permission_bits(
    perms: sdk::models::Permissions,
    cols: (impl thorn::ValueExpr, impl thorn::ValueExpr),
) -> impl thorn::BooleanExpr {
    use thorn::*;

    let perms = perms.to_i64();
    cols.0
        .has_all_bits(perms[0].lit())
        .and(cols.1.has_all_bits(perms[1].lit()))
}
