use serde::de::{self, Deserialize, DeserializeOwned, Deserializer};
use serde::ser::{Serialize, SerializeSeq, Serializer};
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

pub extern crate serde_aux as aux;

pub fn parse<T: FromStr>(s: &str, default: T) -> T {
    s.parse().unwrap_or(default)
}

pub mod hex_key {
    use std::borrow::Cow;

    use super::*;

    pub fn parse_hex_key_inner(key: &mut [u8], value: &str, strict: bool) -> Result<(), Cow<'static, str>> {
        let hex_len = value.len();
        let raw_len = hex_len / 2;

        if hex_len < 32 {
            return Err("Don't use key sizes under 128-bits".into());
        }

        if strict && key.len() * 2 != hex_len {
            return Err(format!("Length mismatch for {}-bit key", key.len() * 8).into());
        }

        match hex::decode_to_slice(value, &mut key[..raw_len]) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Invalid hexidecimal {}-bit encryption key: {e}", key.len() * 8).into()),
        }
    }

    pub mod loose {
        use super::*;

        pub use super::serialize;

        pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
        where
            D: Deserializer<'de>,
            T: Default + AsMut<[u8]>,
        {
            super::deserialize_inner(deserializer, false)
        }
    }

    pub fn serialize<S, T: AsRef<[u8]>>(key: T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        hex::encode(key.as_ref()).serialize(serializer)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: Default + AsMut<[u8]>,
    {
        deserialize_inner(deserializer, true)
    }

    fn deserialize_inner<'de, T, D>(deserializer: D, strict: bool) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: Default + AsMut<[u8]>,
    {
        struct Visitor<T: Default + AsMut<[u8]>>(bool, PhantomData<T>);

        impl<'de, T: Default + AsMut<[u8]>> de::Visitor<'de> for Visitor<T> {
            type Value = T;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("hexidecimal encryption key")
            }

            fn visit_str<E>(self, value: &str) -> Result<T, E>
            where
                E: de::Error,
            {
                let mut key = T::default();

                match parse_hex_key_inner(key.as_mut(), value, self.0) {
                    Ok(_) => Ok(key),
                    Err(e) => Err(E::custom(e)),
                }
            }
        }

        deserializer.deserialize_str(Visitor::<T>(strict, PhantomData))
    }
}

pub mod range {
    use super::*;

    use std::ops::Range;

    pub fn serialize<S, T>(value: &Range<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&value.start)?;
        seq.serialize_element(&value.end)?;
        seq.end()
    }

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Range<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: DeserializeOwned,
    {
        let [start, end] = <[T; 2]>::deserialize(deserializer)?;

        Ok(Range { start, end })
    }
}

pub mod duration {
    use serde::de::SeqAccess;

    use super::*;

    use std::time::Duration;

    pub fn serialize<S>(value: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = value.as_secs();
        let ns = value.subsec_nanos();

        if ns == 0 {
            return s.serialize(serializer);
        }

        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&s)?;
        seq.serialize_element(&ns)?;
        seq.end()
    }

    // pub fn serialize<S>(value: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    // where
    //     S: Serializer,
    // {
    //     serializer.serialize_str(&humantime::format_duration(value.clone()).to_string())
    // }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Duration;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("positive integer for whole seconds or two-element array for [seconds, nanoseconds]")
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Duration, E> {
                Ok(Duration::from_secs(value))
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Duration, E> {
                if value < 0 {
                    Err(E::custom("Negative integer"))
                } else {
                    self.visit_u64(value as u64)
                }
            }

            fn visit_seq<S: SeqAccess<'de>>(self, mut value: S) -> Result<Duration, S::Error> {
                let seconds = match value.next_element::<u64>()? {
                    Some(s) => s,
                    None => return Err(de::Error::custom("Missing seconds value")),
                };

                Ok(Duration::new(seconds, value.next_element::<u32>()?.unwrap_or(0)))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                // TODO: Revisit this. humantime uses 30.44 days per month, which gives ugly results
                humantime::parse_duration(v).map_err(|e| E::custom(e))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

// TODO: Revisit this in a way that actually preserves the values, as bytesize rounds off and loses all precision...
pub mod bytes {
    use super::*;

    use bytesize::ByteSize;

    // pub fn serialize<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    // where
    //     S: Serializer,
    //     T: TryInto<u64> + Copy,
    // {
    //     let bytes = match (*value).try_into() {
    //         Ok(value) => ByteSize(value),
    //         Err(_) => return Err(serde::ser::Error::custom("Could not convert field to u64")),
    //     };

    //     serializer.serialize_str(&bytes.to_string())
    // }

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        T: TryFrom<u64>,
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = u64;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("Unsigned 64-bit integer or byte size specification")
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<u64, E> {
                Ok(value)
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<u64, E> {
                if value < 0 {
                    Err(E::custom("Negative integer"))
                } else {
                    Ok(value as u64)
                }
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<u64, E> {
                match ByteSize::from_str(value) {
                    Ok(value) => Ok(value.0),
                    Err(e) => Err(E::custom(e)),
                }
            }
        }

        deserializer.deserialize_any(Visitor).and_then(|bytes| {
            bytes.try_into().map_err(|_| {
                de::Error::custom(format!("{} cannot fit into {}", bytes, std::any::type_name::<T>()))
            })
        })
    }
}
