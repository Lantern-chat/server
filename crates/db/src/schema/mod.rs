#[macro_export]
macro_rules! cols {
    ($($col:expr),*$(,)?) => {
        std::array::IntoIter::new([$($col),*])
    }
}

pub mod codes;

pub mod tables;
pub use tables::*;

pub use pg::types::Type;
