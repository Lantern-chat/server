use std::{ops::Range, time::Duration};

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Account {
    #[serde(with = "super::util::duration")]
    pub session_duration: Duration,

    /// Minimum user age in years
    pub min_age: u8,

    #[serde(with = "super::util::range")]
    pub password_len: Range<usize>,

    #[serde(with = "super::util::range")]
    pub username_len: Range<usize>,
}

impl Default for Account {
    fn default() -> Self {
        Account {
            session_duration: Duration::from_secs(90 * 24 * 60 * 60), // 3 months / 90 days
            min_age: 13,
            password_len: 8..9999,
            username_len: 3..64,
        }
    }
}
