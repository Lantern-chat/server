use std::{ops::Range, time::Duration};

config::section! {
    #[serde(default)]
    pub struct Account {
        /// Duration of a user session.
        ///
        /// Can be parsed from plain seconds or an array of `[seconds, nanoseconds]`
        ///
        /// Default value is 90 days
        #[serde(with = "config::util::duration")]
        pub session_duration: Duration      = Duration::from_secs(90 * 24 * 60 * 60), // 90 days

        /// Minimum user age in years
        pub min_age: u8                     = 13,

        #[serde(with = "config::util::range")]
        pub password_len: Range<usize>      = 8..9999,

        #[serde(with = "config::util::range")]
        pub username_len: Range<usize>      = 3..64,

        /// Number of MFA/2FA backup codes generated on creation
        pub num_mfa_backups: usize          = 8,

        /// Minutes the MFA/2FA code can be left pending before expiring.
        ///
        /// Default is 30 minutes.
        pub mfa_pending_time: usize = 30,
    }
}

impl Account {
    pub fn configure(&mut self) {
        if self.password_len.start < 8 {
            log::error!("Password length set below 8, defaulting to 8. Use longer passwords.");

            self.password_len.start = 8;
        }
    }
}
