// TODO: Construct a set of "limitations" for each tier of user, which will be combined based on which premium plans they have
pub struct Limitations {
    pub max_message_len: usize,
    pub max_upload_size: u64,
    pub monthly_upload_quota: u64,
}

use std::time::Duration;

config::section! {
    #[serde(default)]
    pub struct Upload {
        pub max_upload_size: u64                = i32::MAX as u64, // 2 GiB
        pub max_upload_chunk_size: i32          = config::MIBIBYTE as i32 * 8, // 8 MiB
        pub monthly_upload_quota: i64           = config::GIBIBYTE as i64 * 1, // 1 GiB
        pub monthly_premium_upload_quota: i64   = config::GIBIBYTE as i64 * 6, // 6 GiB

        pub max_avatar_size: i32                = config::MIBIBYTE as i32 * 4, // 4 MiB
        pub max_banner_size: i32                = config::MIBIBYTE as i32 * 8, // 8 MiB
        pub max_animated_avatar_size: i32       = config::MIBIBYTE as i32 * 5,
        pub max_animated_banner_size: i32       = config::MIBIBYTE as i32 * 20,

        pub avatar_width: u32                   = 256,

        pub banner_width: u32                   = 16 * 40,
        pub banner_height: u32                  =  9 * 40,

        pub max_avatar_pixels: u32              = 1024 * 1024, // 4-byte/32-bit color * 1024^2 = 4 MiB RAM usage
        pub max_banner_pixels: u32              = 2560 * 1440, // 4-byte/32-bit color * 2073600 = 14.0625 MiB RAM usage

        #[serde(deserialize_with = "config::util::aux::container_attributes::deserialize_struct_case_insensitive")]
        pub avatar_formats: UserAssetFormats    = default_avatar_formats(),
        #[serde(deserialize_with = "config::util::aux::container_attributes::deserialize_struct_case_insensitive")]
        pub banner_formats: UserAssetFormats    = default_banner_formats(),

        /// Whether or not to use VP9 WebM's for animated avatars and banners
        ///
        /// VP9 can be significantly slower than h.264 and VP8, perhaps up to 1-2 minutes
        /// for banners. However, the quality and size of VP9 is very good.
        pub vp9_enabled: bool                   = true,

        /// WIP: Not supported yet
        pub av1_enabled: bool                   = false,

        /// Max frame rate for animated avatars and banners
        pub max_fps: f32                        = 24.0,

        /// How often orphaned files should be cleaned
        ///
        /// Can be parsed from plain seconds or an array of `[seconds, nanoseconds]`
        ///
        /// A duration of `0` will disable orphaned file cleanup entirely.
        ///
        /// Minimum interval is 1 minute, default is 1 day
        #[serde(with = "config::util::duration")]
        pub cleanup_interval: Duration          = Duration::from_secs(24 * 60 * 60), // 1 day
    }

    impl Extra {
        fn configure(&mut self) {
            self.avatar_formats.clean();
            self.banner_formats.clean();

            self.avatar_formats.check("avatar", default_avatar_formats);
            self.banner_formats.check("banner", default_banner_formats);

            if !self.cleanup_interval.is_zero() {
                self.cleanup_interval = self.cleanup_interval.max(Duration::from_secs(60));
            }
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
}

impl UserAssetFormats {
    fn static_is_empty(&self) -> bool {
        self.png.is_empty() && self.jpeg.is_empty() && self.avif.is_empty()
    }

    fn check(&mut self, name: &'static str, default: impl FnOnce() -> Self) {
        let d = default();

        if self.static_is_empty() {
            log::warn!("Configuration is missing static {name} file formats, using defaults!");

            self.png = d.png;
            self.jpeg = d.jpeg;
            self.avif = d.avif;
        }
    }

    fn clean(&mut self) {
        self.png.push(100);

        self.png.sort_unstable();
        self.png.dedup();

        self.jpeg.sort_unstable();
        self.jpeg.dedup();

        self.avif.sort_unstable();
        self.avif.dedup();
    }
}

fn default_avatar_formats() -> UserAssetFormats {
    UserAssetFormats {
        png: vec![100],
        jpeg: vec![92, 70, 45],
        avif: vec![95, 80],
    }
}

fn default_banner_formats() -> UserAssetFormats {
    UserAssetFormats {
        png: vec![100],
        jpeg: vec![92, 70, 45],
        avif: vec![90],
    }
}
