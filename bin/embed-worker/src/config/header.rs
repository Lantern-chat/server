use reqwest::header::HeaderValue;

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct DeHeaderValue(pub HeaderValue);

impl std::ops::Deref for DeHeaderValue {
    type Target = HeaderValue;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

use serde::de::{self, Deserialize, Deserializer};

impl<'de> Deserialize<'de> for DeHeaderValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        return deserializer.deserialize_str(Visitor);

        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = DeHeaderValue;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("A valid header value")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match HeaderValue::from_str(v) {
                    Ok(re) => Ok(DeHeaderValue(re)),
                    Err(e) => Err(E::custom(e)),
                }
            }
        }
    }
}
