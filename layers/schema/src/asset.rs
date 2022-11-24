use sdk::api::asset::AssetQuery;
use smol_str::SmolStr;

#[derive(Debug, thiserror::Error)]
pub enum AssetQueryParseError {
    #[error(transparent)]
    IntParseError(#[from] std::num::ParseIntError),

    #[error("Invalid Query")]
    InvalidQuery,
}

pub fn parse(s: &str) -> Result<AssetQuery, AssetQueryParseError> {
    let mut quality = 80;
    let mut animated = true;
    let mut with_alpha = true;
    let mut ext = None;

    for (key, value) in form_urlencoded::parse(s.as_bytes()) {
        match &*key {
            "f" | "flags" => {
                return Ok(AssetQuery::Flags {
                    flags: if let Some(value) = value.strip_suffix("0b") {
                        u16::from_str_radix(value, 2)?
                    } else if let Some(value) = value.strip_prefix("0x") {
                        u16::from_str_radix(value, 16)?
                    } else {
                        u16::from_str_radix(&value, 10)?
                    },
                });
            }
            "q" | "quality" => quality = value.parse()?,
            "animated" => animated = util::parse_boolean(&value)?,
            "alpha" | "with_alpha" => with_alpha = util::parse_boolean(&value)?,
            "ext" | "format" if !value.is_empty() => ext = Some(SmolStr::from(value)),
            _ => {}
        }
    }

    Ok(AssetQuery::HumanReadable {
        quality,
        animated,
        with_alpha,
        ext,
    })
}
