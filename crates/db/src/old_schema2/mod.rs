use pg::types::Type;
use sea_query::*;

pub fn value_to_type(v: Value) -> Option<Type> {
    Some(match v {
        Value::Bool(_) => Type::BOOL,
        Value::TinyInt(_)
        | Value::SmallInt(_)
        | Value::TinyUnsigned(_)
        | Value::SmallUnsigned(_) => Type::INT2,
        Value::Int(_) | Value::Unsigned(_) => Type::INT4,
        Value::BigInt(_) | Value::BigUnsigned(_) => Type::INT8,
        Value::Float(_) => Type::FLOAT4,
        Value::Double(_) => Type::FLOAT8,
        Value::String(_) => Type::TEXT,
        Value::Bytes(_) => Type::BYTEA,
        Value::Json(_) => Type::JSON,
        Value::DateTime(_) => Type::TIMESTAMP,
        Value::Uuid(_) => Type::UUID,
        _ => return None,
    })
}

#[macro_export]
macro_rules! cols {
    ($($col:expr),*$(,)?) => {
        std::array::IntoIter::new([$($col),*])
    }
}

pub mod tables;

use self::tables::*;
