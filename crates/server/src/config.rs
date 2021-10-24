use std::path::PathBuf;
use std::{env, time::Duration};

use std::ops::Range;

use aes::{cipher::BlockCipherKey, Aes256};

#[derive(Debug, Clone)]
pub struct LanternConfig {
    pub num_parallel_tasks: usize,
    pub login_session_duration: Duration,
    pub min_user_age_in_years: u8,
    pub password_len: Range<usize>,
    pub username_len: Range<usize>,
    pub partyname_len: Range<usize>,
    pub roomname_len: Range<usize>,
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

    /// AES-128 key for encrypting snowflakes
    pub sf_key: u128,

    pub data_path: PathBuf,

    pub cert_path: PathBuf,
    pub key_path: PathBuf,
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
            file_key: {
                let mut key: BlockCipherKey<Aes256> = BlockCipherKey::<Aes256>::default();

                hex::decode_to_slice(env::var("FS_KEY").unwrap(), key.as_mut_slice())
                    .expect("Invalid hexidecimal AES256 Key: FS_KEY");

                key
            },
            mfa_key: {
                let mut key: BlockCipherKey<Aes256> = BlockCipherKey::<Aes256>::default();

                hex::decode_to_slice(env::var("MFA_KEY").unwrap(), key.as_mut_slice())
                    .expect("Invalid hexidecimal AES256 Key: MFA_KEY");

                key
            },
            sf_key: {
                let mut key = [0u8; 16];

                hex::decode_to_slice(env::var("SF_KEY").unwrap(), &mut key)
                    .expect("Invalid hexidecimal AES128 Key: SF_KEY");

                u128::from_le_bytes(key)
            },
            data_path: { PathBuf::from(env::var("DATA_DIR").unwrap()) },
            cert_path: { PathBuf::from(env::var("CERT_PATH").unwrap()) },
            key_path: { PathBuf::from(env::var("KEY_PATH").unwrap()) },
        }
    }
}
