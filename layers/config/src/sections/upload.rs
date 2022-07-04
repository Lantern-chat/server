// TODO: Construct a set of "limitations" for each tier of user, which will be combined based on which premium plans they have
pub struct Limitations {
    pub max_message_len: usize,
    pub max_upload_size: u64,
    pub monthly_upload_quota: u64,
}

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
    }
}
