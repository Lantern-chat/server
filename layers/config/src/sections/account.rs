use std::{ops::Range, time::Duration};

section! {
    #[serde(default)]
    pub struct Account {
        /// Duration of a user session.
        ///
        /// Can be parsed from plain seconds or an array of `[seconds, nanoseconds]`
        ///
        /// Default value is 90 days
        #[serde(with = "super::util::duration")]
        pub session_duration: Duration      = Duration::from_secs(90 * 24 * 60 * 60), // 90 days

        /// Minimum user age in years
        pub min_age: u8                     = 13,

        #[serde(with = "super::util::range")]
        pub password_len: Range<usize>      = 8..9999,

        #[serde(with = "super::util::range")]
        pub username_len: Range<usize>      = 3..64,
    }
}
