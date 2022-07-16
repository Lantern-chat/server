// TODO: Construct a set of "limitations" for each tier of user, which will be combined based on which premium plans they have
pub struct Limitations {
    pub max_message_len: usize,
    pub max_upload_size: u64,
    pub monthly_upload_quota: u64,
}

use std::time::Duration;

section! {
    #[serde(default)]
    pub struct Upload {
        pub max_upload_size: u64                = i32::MAX as u64, // 2 GiB
        pub max_upload_chunk_size: i32          = crate::MIBIBYTE * 8, // 8 MiB
        pub monthly_upload_quota: i64           = crate::GIBIBYTE * 1, // 1 GiB
        pub monthly_premium_upload_quota: i64   = crate::GIBIBYTE * 6, // 6 GiB
        pub max_avatar_size: i32                = crate::MIBIBYTE * 4, // 4 MiB
        pub max_banner_size: i32                = crate::MIBIBYTE * 8, // 8 MiB

        pub avatar_width: u32                   = 256,

        pub banner_width: u32                   = 1600,
        pub banner_height: u32                  = 900,

        pub max_avatar_pixels: u32              = 1024 * 1024,  // 4-byte/32-bit color * 1024^2 = 4 MiB RAM usage
        pub max_banner_pixels: u32              = 2560 * 1440, // 4-byte/32-bit color * 2073600 = 14.0625 MiB RAM usage

        #[serde(deserialize_with = "serde_aux::container_attributes::deserialize_struct_case_insensitive")]
        pub avatar_formats: UserAssetFormats    = default_avatar_formats(),
        #[serde(deserialize_with = "serde_aux::container_attributes::deserialize_struct_case_insensitive")]
        pub banner_formats: UserAssetFormats    = default_banner_formats(),

        /// How often orphaned files should be cleaned
        ///
        /// Can be parsed from plain seconds or an array of `[seconds, nanoseconds]`
        ///
        /// A duration of `0` will disable orphaned file cleanup entirely.
        ///
        /// Minimum interval is 1 minute, default is 1 day
        #[serde(with = "super::util::duration")]
        pub cleanup_interval: Duration          = Duration::from_secs(24 * 60 * 60), // 1 day
    }
}

impl Upload {
    pub fn configure(&mut self) {
        self.avatar_formats.clean();
        self.banner_formats.clean();

        self.avatar_formats.check("avatar", default_avatar_formats);
        self.banner_formats.check("banner", default_banner_formats);

        if !self.cleanup_interval.is_zero() {
            self.cleanup_interval = self.cleanup_interval.max(Duration::from_secs(60));
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UserAssetFormats {
    pub png: Vec<u8>,
    pub jpeg: Vec<u8>,
    /// NOTE: The actual quality of AVIF encoding has been adjusted to more closely match JPEG quality,
    /// so use numbers here that would be similar to JPEG.
    pub avif: Vec<u8>,
    pub gif: Vec<u8>,
    pub webm: Vec<u8>,
}

impl UserAssetFormats {
    fn static_is_empty(&self) -> bool {
        self.png.is_empty() && self.jpeg.is_empty() && self.avif.is_empty()
    }

    fn animated_is_empty(&self) -> bool {
        self.gif.is_empty() && self.webm.is_empty()
    }

    fn check(&mut self, name: &'static str, default: impl FnOnce() -> Self) {
        let d = default();

        if self.static_is_empty() {
            log::warn!("Configuration is missing static {name} file formats, using defaults!");

            self.png = d.png;
            self.jpeg = d.jpeg;
            self.avif = d.avif;
        }

        if self.animated_is_empty() {
            log::warn!("Configuration is missing animated {name} file formats, using defaults!");

            self.gif = d.gif;
            self.webm = d.webm;
        }
    }

    fn clean(&mut self) {
        self.png.sort_unstable();
        self.png.dedup();

        self.jpeg.sort_unstable();
        self.jpeg.dedup();

        self.avif.sort_unstable();
        self.avif.dedup();

        self.gif.sort_unstable();
        self.gif.dedup();

        self.webm.sort_unstable();
        self.webm.dedup();
    }
}

fn default_avatar_formats() -> UserAssetFormats {
    UserAssetFormats {
        png: vec![100],
        jpeg: vec![92, 70, 45],
        avif: vec![95, 80],
        gif: vec![100],
        webm: vec![95],
    }
}

fn default_banner_formats() -> UserAssetFormats {
    UserAssetFormats {
        png: vec![100],
        jpeg: vec![92, 70, 45],
        avif: vec![90],
        gif: vec![95],
        webm: vec![95],
    }
}
