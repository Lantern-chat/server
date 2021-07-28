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
    pub max_message_newlines: usize,
    pub max_upload_size: i32,
    pub file_key: BlockCipherKey<Aes256>,
}

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
            message_len: 1..5000,
            max_message_newlines: 120,
            max_upload_size: 1024 * 1024 * 8,
            file_key: {
                let mut key: BlockCipherKey<Aes256> = BlockCipherKey::<Aes256>::default();

                hex::decode_to_slice(env::var("FS_KEY").unwrap(), key.as_mut_slice())
                    .expect("Invalid hexidecimal AES256 Key");

                key
            },
        }
    }
}
