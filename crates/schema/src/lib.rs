#[macro_export]
macro_rules! cols {
    ($($col:expr),*$(,)?) => {
        std::array::IntoIter::new([$($col),*])
    }
}

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
