// TODO: Construct a set of "limitations" for each tier of user, which will be combined based on which premium plans they have
pub struct Limitations {
    pub max_message_len: usize,
    pub max_upload_size: u64,
    pub monthly_upload_quota: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Upload {
    pub max_upload_size: u64,
    pub max_upload_chunk_size: i32,
    pub monthly_upload_quota: i64,
    pub monthly_premium_upload_quota: i64,
    pub max_avatar_size: i32,
    pub max_avatar_pixels: u32,
    pub max_avatar_width: u32,
}

const KIBIBYTE: i32 = 1024;
const MIBIBYTE: i32 = KIBIBYTE * 1024;
const GIBIBYTE: i64 = MIBIBYTE as i64 * 1024;

impl Default for Upload {
    fn default() -> Upload {
        Upload {
            max_upload_size: i32::MAX as u64,
            max_upload_chunk_size: MIBIBYTE * 8,        // 8 MiB
            max_avatar_size: MIBIBYTE * 2,              // 2 MiB
            monthly_upload_quota: GIBIBYTE * 1,         // 1 GiB
            monthly_premium_upload_quota: GIBIBYTE * 6, // 6 GiB
            max_avatar_pixels: 1024 * 1024,             // 32-bit color * 1024^2 = 2.56 MiB RAM usage
            max_avatar_width: 256,
        }
    }
}
