use std::time::Duration;

use std::ops::Range;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanternConfig {
    pub login_session_duration: Duration,
    pub min_user_age_in_years: u8,
    pub password_len: Range<usize>,
    pub username_len: Range<usize>,
    pub partyname_len: Range<usize>,
    pub roomname_len: Range<usize>,
    pub message_len: Range<usize>,
    pub max_message_newlines: usize,
    pub max_upload_size: u32,
}

impl Default for LanternConfig {
    fn default() -> Self {
        LanternConfig {
            login_session_duration: Duration::from_secs(90 * 24 * 60 * 60), // 3 months
            min_user_age_in_years: 13,
            password_len: 8..9999,
            username_len: 3..64,
            partyname_len: 3..64,
            roomname_len: 3..64,
            message_len: 1..5000,
            max_message_newlines: 120,
            max_upload_size: 1024 * 1024 * 8,
        }
    }
}
