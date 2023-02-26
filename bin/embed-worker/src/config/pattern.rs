use regex::Regex;

#[derive(Debug, Clone)]
pub struct Pattern(Regex);

impl std::ops::Deref for Pattern {
    type Target = Regex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

use serde::de::{self, Deserialize, Deserializer};

impl<'de> Deserialize<'de> for Pattern {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        return deserializer.deserialize_str(Visitor);

        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Pattern;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("A valid regular expression")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match Regex::new(v) {
                    Ok(re) => Ok(Pattern(re)),
                    Err(e) => Err(E::custom(e)),
                }
            }
        }
    }
}
