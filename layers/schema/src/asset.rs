bitflags::bitflags! {
    pub struct AssetFlags: i16 {
        /// 7-bit unsigned integer for quality from `[0-128)`
        ///
        /// A quality value greater then 100 indicates some lossless encoding
        const QUALITY  = 127;

        /// Indicates if the encoded image is animated
        const ANIMATED = 1 << 8;

        /// Reserved for some potentially useful flag
        const RESERVED = 1 << 9;

        const FORMAT_PNG  = 1 << 10;
        const FORMAT_JPEG = 1 << 11;
        const FORMAT_GIF  = 1 << 12;
        const FORMAT_AVIF = 1 << 13;

        const FORMATS = Self::FORMAT_PNG.bits | Self::FORMAT_JPEG.bits | Self::FORMAT_GIF.bits | Self::FORMAT_AVIF.bits;
    }
}

impl AssetFlags {
    pub fn with_quality(mut self, q: u8) -> Self {
        self.remove(Self::QUALITY);
        self | AssetFlags::from_bits_truncate(q.min(127) as i16)
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

        AssetFlags::FORMAT_JPEG
    }

    pub fn with_ext(self, ext: &str) -> Self {
        self | Self::from_ext(ext)
    }
}
