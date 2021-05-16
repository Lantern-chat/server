#[macro_export]
macro_rules! cols {
    ($($col:expr),*$(,)?) => {
        std::array::IntoIter::new([$($col),*])
    }
}

pub mod tables;
pub use tables::*;

pub use tokio_postgres::types::Type;
