bitflags::bitflags! {
    /// NOTE: Formats are as individual bitflags (rather than some integer value) so we can do
    /// simpler queries when matching valid formats. A user can select formats A, B and C, and testing for a match
    /// can be done with a single bitwise-AND operation, rather than many comparisons or an `IN ARRAY` operation.
    pub struct AssetFlags: i16 {
        /// 7-bit unsigned integer for quality from `[0-128)`
        ///
        /// A quality value greater then 100 indicates some lossless encoding
        const QUALITY  = 127;

        const HAS_ALPHA = 1 << 8;

        /// Indicates if the encoded image is animated
        const ANIMATED = 1 << 9;

        const FORMAT_PNG  = 1 << 10;
        const FORMAT_JPEG = 1 << 11;
        const FORMAT_GIF  = 1 << 12;
        const FORMAT_AVIF = 1 << 13;
        const FORMAT_WEBM = 1 << 14;

        const FORMATS = Self::FORMAT_PNG.bits | Self::FORMAT_JPEG.bits | Self::FORMAT_GIF.bits | Self::FORMAT_AVIF.bits | Self::FORMAT_WEBM.bits;

        const MAYBE_UNSUPPORTED_FORMATS = Self::FORMAT_AVIF.bits;

        const FLAGS = Self::HAS_ALPHA.bits | Self::ANIMATED.bits;

        const FORMATS_AND_FLAGS = Self::FORMATS.bits | Self::FLAGS.bits;
    }
}

impl AssetFlags {
    pub const fn with_quality(self, q: u8) -> Self {
        self.intersection(Self::QUALITY.complement()).union(if q < 128 {
            AssetFlags::from_bits_truncate(q as i16)
        } else {
            AssetFlags::QUALITY
        })
    }

    pub const fn with_alpha(&self, has_alpha: bool) -> Self {
        if has_alpha {
            self.union(Self::HAS_ALPHA)
        } else {
            self.difference(Self::HAS_ALPHA)
        }
    }

    pub const fn quality(&self) -> u8 {
        self.intersection(Self::QUALITY).bits as u8
    }

    pub fn from_ext(ext: &str) -> Self {
        static FORMAT_EXTS: &[(AssetFlags, &'static str)] = &[
            (AssetFlags::FORMAT_PNG, "png"),
            (AssetFlags::FORMAT_JPEG, "jpeg"),
            (AssetFlags::FORMAT_JPEG, "jpg"),
            (AssetFlags::FORMAT_GIF, "gif"),
            (AssetFlags::FORMAT_AVIF, "avif"),
        ];

        // bailout on obviously invalid extensions
        if 3 <= ext.len() && ext.len() < 5 {
            for &(f, e) in FORMAT_EXTS {
                if ext.eq_ignore_ascii_case(e) {
                    return f;
                }
            }
        }

        AssetFlags::empty()
    }

    pub fn with_ext(self, ext: &str) -> Self {
        self | Self::from_ext(ext)
    }
}

use sdk::api::AssetQuery;
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
                    flags: if value.starts_with("0b") {
                        u16::from_str_radix(&value[2..], 2)?
                    } else if value.starts_with("0x") {
                        u16::from_str_radix(&value[2..], 16)?
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
