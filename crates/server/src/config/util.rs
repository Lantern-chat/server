use serde::de::{self, Deserialize, DeserializeOwned, Deserializer};
use serde::ser::{Serialize, SerializeSeq, Serializer};
use std::fmt;
use std::marker::PhantomData;

pub mod hex_key {
    use super::*;

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
        struct Visitor<T: Default + AsMut<[u8]>>(PhantomData<T>);

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

                let len = key.as_mut().len();

                if value.len() != key.as_mut().len() * 2 {
                    return Err(E::custom(format!("Length mismatch for {}-bit key", len * 8)));
                }

                match hex::decode_to_slice(value, key.as_mut()) {
                    Ok(_) => Ok(key),
                    Err(e) => Err(E::custom(e)),
                }
            }
        }

        deserializer.deserialize_str(Visitor::<T>(PhantomData))
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
    use super::*;

    use std::time::Duration;

    pub fn serialize<S>(value: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&value.as_secs())?;
        seq.serialize_element(&value.subsec_nanos())?;
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        <[u64; 2]>::deserialize(deserializer).map(|[secs, ns]| Duration::new(secs, ns as u32))
    }
}

// TODO: Revisit this. humantime uses 30.44 days per month, which gives ugly results
/*
pub mod duration {
    use super::*;

    use std::time::Duration;

    pub fn serialize<S>(value: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&humantime::format_duration(value.clone()).to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Duration;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("duration string such as \"2d 1m 43s\"")
            }

            fn visit_str<E>(self, value: &str) -> Result<Duration, E>
            where
                E: de::Error,
            {
                humantime::parse_duration(value).map_err(|e| E::custom(e))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}
 */

// TODO: Revisit this in a way that actually preserves the values, as bytesize rounds off and loses all precision...
/*
pub mod bytes {
    use super::*;

    use bytesize::ByteSize;

    pub fn serialize<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: TryInto<u64> + Copy,
    {
        let bytes = match (*value).try_into() {
            Ok(value) => ByteSize(value),
            Err(_) => return Err(serde::ser::Error::custom("Could not convert field to u64")),
        };

        serializer.serialize_str(&bytes.to_string())
    }

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
                use std::str::FromStr;

                match ByteSize::from_str(value) {
                    Ok(value) => Ok(value.0),
                    Err(e) => Err(E::custom(e)),
                }
            }
        }

        deserializer.deserialize_any(Visitor).and_then(|bytes| {
            bytes.try_into().map_err(|_| {
                de::Error::custom(format!(
                    "{} cannot fit into {}",
                    bytes,
                    std::any::type_name::<T>()
                ))
            })
        })
    }
}
*/
