#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i8)]
pub enum AssetFileFormat {
    Jpeg = 0,
    Png = 1,
    Gif = 2,
    Avif = 3,

    __MAX,
}

static FORMAT_EXTS: &[(AssetFileFormat, &'static str)] = &[
    (AssetFileFormat::Png, "png"),
    (AssetFileFormat::Jpeg, "jpeg"),
    (AssetFileFormat::Jpeg, "jpg"),
    (AssetFileFormat::Gif, "gif"),
    (AssetFileFormat::Avif, "avif"),
];

impl AssetFileFormat {
    pub fn from_ext(ext: &str) -> Self {
        // bailout on obviously invalid extensions
        if 3 <= ext.len() && ext.len() < 5 {
            for &(f, e) in FORMAT_EXTS {
                if ext.eq_ignore_ascii_case(e) {
                    return f;
                }
            }
        }

        AssetFileFormat::Jpeg
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
#[repr(i8)]
pub enum AssetQualityLevel {
    Lossless = 0,
    High = 1,

    /// NOTE: This may be used instead of Medium in cases where JPEG would destroy lines
    MediumHigh = 2,
    Medium = 3,
    Low = 4,
    VeryLow = 5,

    __MAX,
}

static QUALITY_MAP: &[(AssetQualityLevel, u8)] = &[
    // NOTE: Even JPEG-100 is not truly lossless, but for a JPEG
    // version it's the best we can do
    (AssetQualityLevel::Lossless, 100),
    (AssetQualityLevel::High, 98),
    (AssetQualityLevel::MediumHigh, 92),
    (AssetQualityLevel::Medium, 85),
    (AssetQualityLevel::Low, 75),
    (AssetQualityLevel::VeryLow, 40),
];

impl AssetQualityLevel {
    pub fn to_jpeg_quality(self) -> u8 {
        if self < Self::__MAX {
            QUALITY_MAP[self as usize].1
        } else {
            100 // highest quality, just in case
        }
    }

    /// Select a Quality Level from the given JPEG quality, rounding to the nearest level
    pub fn from_jpeg_quality(q: u8) -> Self {
        for &(l, j) in QUALITY_MAP {
            if q >= j {
                if l != AssetQualityLevel::Lossless {
                    // get previous quality level
                    let (pl, pj) = QUALITY_MAP[l as usize - 1];

                    // choose closest
                    if (pj - q) < (q - j) {
                        return pl;
                    }
                }

                return l;
            }
        }

        Self::VeryLow
    }
}

// ensure both can fit within a nibble
static_assertions::const_assert!((AssetFileFormat::__MAX as i8) < 16);
static_assertions::const_assert!((AssetQualityLevel::__MAX as i8) < 16);

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AssetFlags(pub i16);

impl AssetFlags {
    /// Format specifier goes in lowest 4 bits
    const FORMAT_MASK: i16 = 0x0F;
    /// Quality specifier goes in the high 4 bits of the lowest byte
    const QUALITY_MASK: i16 = 0xF0;

    #[inline]
    pub fn format(&self) -> AssetFileFormat {
        let format = self.0 & Self::FORMAT_MASK;

        if format < AssetFileFormat::__MAX as i16 {
            return unsafe { std::mem::transmute(format as i8) };
        }

        panic!("Invalid file format");
    }

    #[inline]
    pub fn quality(&self) -> AssetQualityLevel {
        let quality = (self.0 & Self::QUALITY_MASK) >> 4;

        if quality < AssetQualityLevel::__MAX as i16 {
            return unsafe { std::mem::transmute(quality as i8) };
        }

        panic!("Invalid quality level");
    }

    #[inline]
    pub fn with_format(mut self, format: AssetFileFormat) -> Self {
        // clear any format bits then assign new bits
        self.0 = (self.0 & !Self::FORMAT_MASK) | format as i16;
        self
    }

    #[inline]
    pub fn with_quality(mut self, quality: AssetQualityLevel) -> Self {
        // clear any quality bits then assign new bits, shifted into position
        self.0 = (self.0 & !Self::QUALITY_MASK) | ((quality as i16) << 4);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{AssetQualityLevel, QUALITY_MAP};

    #[test]
    #[rustfmt::skip]
    fn test_asset_quality_level() {
        for &(l, q) in QUALITY_MAP {
            assert_eq!(l.to_jpeg_quality(), q);
        }

        assert_eq!(AssetQualityLevel::from_jpeg_quality(103), AssetQualityLevel::Lossless);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(102), AssetQualityLevel::Lossless);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(101), AssetQualityLevel::Lossless);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(100), AssetQualityLevel::Lossless); //
        assert_eq!(AssetQualityLevel::from_jpeg_quality(99), AssetQualityLevel::High);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(98), AssetQualityLevel::High); //
        assert_eq!(AssetQualityLevel::from_jpeg_quality(97), AssetQualityLevel::High);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(96), AssetQualityLevel::High);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(95), AssetQualityLevel::MediumHigh);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(94), AssetQualityLevel::MediumHigh);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(93), AssetQualityLevel::MediumHigh);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(92), AssetQualityLevel::MediumHigh); //
        assert_eq!(AssetQualityLevel::from_jpeg_quality(91), AssetQualityLevel::MediumHigh);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(90), AssetQualityLevel::MediumHigh);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(89), AssetQualityLevel::MediumHigh);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(88), AssetQualityLevel::Medium);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(87), AssetQualityLevel::Medium);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(85), AssetQualityLevel::Medium); //
        assert_eq!(AssetQualityLevel::from_jpeg_quality(84), AssetQualityLevel::Medium);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(83), AssetQualityLevel::Medium);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(82), AssetQualityLevel::Medium);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(81), AssetQualityLevel::Medium);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(80), AssetQualityLevel::Low);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(79), AssetQualityLevel::Low);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(78), AssetQualityLevel::Low);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(77), AssetQualityLevel::Low);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(76), AssetQualityLevel::Low);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(75), AssetQualityLevel::Low); //
        assert_eq!(AssetQualityLevel::from_jpeg_quality(74), AssetQualityLevel::Low);
        assert_eq!(AssetQualityLevel::from_jpeg_quality(73), AssetQualityLevel::Low);
    }
}
