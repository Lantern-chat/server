use std::path::PathBuf;
use std::{env, time::Duration};

use std::ops::Range;

use aes::{cipher::BlockCipherKey, Aes128, Aes256};
use schema::auth::BotTokenKey;

#[derive(Debug, Clone)]
pub struct LanternConfig {
    pub num_parallel_tasks: usize,
    pub login_session_duration: Duration,
    pub min_user_age_in_years: u8,
    pub password_len: Range<usize>,
    pub username_len: Range<usize>,
    pub partyname_len: Range<usize>,
    pub roomname_len: Range<usize>,
    pub max_newlines: usize,
    pub message_len: Range<usize>,
    pub premium_message_len: Range<usize>,
    pub max_message_newlines: usize,
    pub max_upload_size: u64,
    pub max_upload_chunk_size: i32,
    pub monthly_upload_quota: i64,
    pub monthly_premium_upload_quota: i64,
    pub max_avatar_size: i32,
    pub max_avatar_pixels: u32,
    pub max_avatar_width: u32,
    pub file_key: BlockCipherKey<Aes256>,
    pub mfa_key: BlockCipherKey<Aes256>,
    pub sf_key: BlockCipherKey<Aes128>,
    pub bt_key: BotTokenKey,

    pub data_path: PathBuf,

    pub cert_path: PathBuf,
    pub key_path: PathBuf,

    pub hcaptcha_secret: String,
    pub hcaptcha_sitekey: String,
}

const KIBIBYTE: i32 = 1024;
const MIBIBYTE: i32 = KIBIBYTE * 1024;
const GIBIBYTE: i64 = MIBIBYTE as i64 * 1024;

impl Default for LanternConfig {
    fn default() -> Self {
        LanternConfig {
            num_parallel_tasks: num_cpus::get(),
            login_session_duration: Duration::from_secs(90 * 24 * 60 * 60), // 3 months
            min_user_age_in_years: 13,
            password_len: 8..9999,
            username_len: 3..64,
            partyname_len: 3..64,
            roomname_len: 3..64,
            max_newlines: 80,
            message_len: 1..2500,
            premium_message_len: 1..5000,
            max_message_newlines: 120,
            max_upload_size: i32::MAX as u64,
            max_upload_chunk_size: MIBIBYTE * 8,        // 8 MiB
            max_avatar_size: MIBIBYTE * 2,              // 2 MiB
            monthly_upload_quota: GIBIBYTE * 1,         // 1 GiB
            monthly_premium_upload_quota: GIBIBYTE * 6, // 6 GiB
            max_avatar_pixels: 1024 * 1024,             // 32-bit color * 1024^2 = 2.56 MiB RAM usage
            max_avatar_width: 256,
            file_key: read_hex_key("FS_KEY", true).into(),
            mfa_key: read_hex_key("MFA_KEY", true).into(),
            sf_key: read_hex_key("SF_KEY", true).into(),
            bt_key: read_hex_key("BT_KEY", false).into(),
            data_path: { PathBuf::from(env::var("DATA_DIR").unwrap()) },
            cert_path: { PathBuf::from(env::var("CERT_PATH").unwrap()) },
            key_path: { PathBuf::from(env::var("KEY_PATH").unwrap()) },
            hcaptcha_secret: { env::var("HCAPTCHA_SECRET").unwrap() },
            hcaptcha_sitekey: { env::var("HCAPTCHA_SITEKEY").unwrap() },
        }
    }
}

fn read_hex_key<const N: usize>(name: &'static str, strict: bool) -> [u8; N] {
    let value = env::var(name).unwrap_or_else(|_| panic!("Missing environment variable \"{}\"", name));
    let hex_len = value.len();

    if hex_len < 32 {
        panic!("Don't use key sizes under 128-bits for key: \"{}\"", name);
    }

    let mut key = [0; N];
    if strict && key.len() * 2 != hex_len {
        panic!("Length mismatch for {}-bit key: \"{}\"", N * 8, name);
    }

    hex::decode_to_slice(value, &mut key[..hex_len / 2])
        .unwrap_or_else(|_| panic!("Invalid hexidecimal {}-bit encryption key: \"{}\"", N * 8, name));

    key
}
