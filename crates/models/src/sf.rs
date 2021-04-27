use std::{
    fmt,
    str::FromStr,
    time::{Duration, SystemTime},
};

use std::num::NonZeroU64;

/**
    Snowflakes are a UUID-like system designed to embed timestamp information in a monotonic format.

    This implementation provides an EPOCH for offsetting the timestamp, and implementations for SQL/JSON interop.
*/
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Snowflake(pub NonZeroU64);

/// Arbitrarily chosen starting epoch to offset the clock by
pub const LANTERN_EPOCH: u64 = 1550102400000;

impl Snowflake {
    #[inline]
    pub const fn null() -> Snowflake {
        Snowflake(unsafe { NonZeroU64::new_unchecked(1) })
    }

    /// Gets the number of milliseconds since the unix epoch
    #[inline]
    pub fn epoch_ms(&self) -> u64 {
        self.raw_timestamp() + LANTERN_EPOCH
    }

    #[inline]
    pub fn timestamp(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_millis(self.epoch_ms())
    }

    #[inline]
    pub fn raw_timestamp(&self) -> u64 {
        self.0.get() >> 22
    }

    #[inline]
    pub const fn to_u64(self) -> u64 {
        self.0.get()
    }
}

impl FromStr for Snowflake {
    type Err = <NonZeroU64 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        NonZeroU64::from_str(s).map(Snowflake)
    }
}

impl fmt::Display for Snowflake {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(feature = "tokio-pg")]
mod pg_impl {
    use super::*;

    use std::error::Error;

    use bytes::BytesMut;
    use postgres_types::{FromSql, IsNull, ToSql, Type};

    impl<'a> FromSql<'a> for Snowflake {
        fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
            <i64 as FromSql<'a>>::from_sql(ty, raw)
                .map(|raw| Snowflake(NonZeroU64::new(raw as u64).unwrap()))
        }

        fn accepts(ty: &Type) -> bool {
            <i64 as FromSql<'a>>::accepts(ty)
        }
    }

    impl ToSql for Snowflake {
        fn to_sql(
            &self,
            ty: &Type,
            out: &mut BytesMut,
        ) -> Result<IsNull, Box<dyn Error + Sync + Send>>
        where
            Self: Sized,
        {
            let value = self.0.get() as i64;
            value.to_sql(ty, out)
        }

        fn accepts(ty: &Type) -> bool
        where
            Self: Sized,
        {
            <i64 as ToSql>::accepts(ty)
        }

        fn to_sql_checked(
            &self,
            ty: &Type,
            out: &mut BytesMut,
        ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
            let value = self.0.get() as i64;
            value.to_sql_checked(ty, out)
        }
    }
}

mod serde_impl {
    use super::*;

    use std::str::FromStr;

    use serde::de::{Deserialize, Deserializer, Error, Visitor};
    use serde::ser::{Serialize, Serializer};

    impl Serialize for Snowflake {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.collect_str(self)
        }
    }

    impl<'de> Deserialize<'de> for Snowflake {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct SnowflakeVisitor;

            impl<'de> Visitor<'de> for SnowflakeVisitor {
                type Value = Snowflake;

                fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    f.write_str("a 64-bit integer or numeric string")
                }

                fn visit_u64<E: Error>(self, v: u64) -> Result<Self::Value, E> {
                    match NonZeroU64::new(v) {
                        Some(x) => Ok(Snowflake(x)),
                        None => Err(E::custom("expected a non-zero value")),
                    }
                }

                fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                    Snowflake::from_str(v)
                        .map_err(|e| E::custom(&format!("Invalid Snowflake: {}", e)))
                }
            }

            deserializer.deserialize_any(SnowflakeVisitor)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sf_size() {
        use std::mem::size_of;
        assert_eq!(size_of::<u64>(), size_of::<Option<Snowflake>>());
    }

    #[test]
    fn test_serde() {
        use serde_derive::*;

        #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
        struct Nested {
            x: Snowflake,
        }

        let _: Snowflake = serde_json::from_str(r#""12234""#).unwrap();
        let _: Snowflake = serde_json::from_str(r#"12234"#).unwrap();
        let _: Nested = serde_json::from_str(r#"{"x": 12234}"#).unwrap();
        let _: Nested = serde_json::from_str(r#"{"x": "12234"}"#).unwrap();
    }
}
