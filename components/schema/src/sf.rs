use std::time::{Duration, SystemTime};

use std::num::NonZeroU64;

use aes::{cipher::Key, Aes128};

pub use sdk::models::sf::LANTERN_EPOCH;
pub use snowflake::{generator::Generator as SnowflakeGenerator, Snowflake};

use sdk::models::{Timestamp, LANTERN_EPOCH_PDT};

pub trait SnowflakeExt {
    #[inline]
    fn from_i64(id: i64) -> Option<Snowflake> {
        NonZeroU64::new(id as u64).map(Snowflake)
    }

    // Constructs a Snowflake from the given timestamp with any of the deduplication
    // values. This is ideal for database searches using simple operators.
    fn timestamp_only(ts: SystemTime) -> Snowflake {
        let elapsed: Duration = ts.duration_since(SystemTime::UNIX_EPOCH).unwrap();
        let ms = elapsed.as_millis() as u64 - LANTERN_EPOCH;

        Snowflake(unsafe { NonZeroU64::new_unchecked(ms << 22) })
    }

    /// **WARNING**: DO NOT USE FOR UNIQUE IDS
    #[inline]
    fn at_ts(ts: Timestamp) -> Snowflake {
        Snowflake::from_unix_ms(
            LANTERN_EPOCH,
            ts.duration_since(Timestamp::UNIX_EPOCH).whole_milliseconds().max(0) as u64,
        )
        .unwrap()
    }

    /// **WARNING**: DO NOT USE FOR UNIQUE IDS
    #[inline]
    fn at(ts: SystemTime) -> Snowflake {
        Snowflake::from_unix_ms(
            LANTERN_EPOCH,
            ts.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64,
        )
        .unwrap()
    }

    /// **WARNING**: DO NOT USE FOR UNIQUE IDS
    #[inline]
    fn at_date(ts: time::Date) -> Snowflake {
        Snowflake::from_unix_ms(
            LANTERN_EPOCH,
            (ts - LANTERN_EPOCH_PDT.date()).whole_milliseconds() as u64,
        )
        .unwrap()
    }

    fn add(self, duration: time::Duration) -> Option<Snowflake>;

    fn encrypt(self, key: Key<Aes128>) -> u128;

    #[inline]
    fn decrypt(block: u128, key: Key<Aes128>) -> Option<Snowflake> {
        use aes::cipher::{BlockDecrypt, KeyInit};

        let mut block = unsafe { std::mem::transmute(block) };

        let cipher = Aes128::new(&key);

        cipher.decrypt_block(&mut block);

        let [l, _]: [u64; 2] = unsafe { std::mem::transmute(block) };

        NonZeroU64::new(l).map(Snowflake)
    }
}

impl SnowflakeExt for Snowflake {
    fn add(self, duration: time::Duration) -> Option<Snowflake> {
        let value = self.0.get();
        let offset = duration.whole_milliseconds() as i64;

        let mut raw_ts = value >> 22;
        if offset < 0 {
            raw_ts = raw_ts.saturating_sub(-offset as u64);
        } else {
            raw_ts = raw_ts.saturating_add(offset as u64);
        }

        const NON_TIMESTAMP_MASK: u64 = u64::MAX >> 42; // shift in zeroes from above so only timestamp bits are 0

        // combine new timestamp with the old non-timestamp bits
        NonZeroU64::new((NON_TIMESTAMP_MASK & value) | (raw_ts << 22)).map(Snowflake)
    }

    #[inline]
    fn encrypt(self, key: Key<Aes128>) -> u128 {
        use aes::cipher::{BlockEncrypt, KeyInit};

        let mut block = unsafe { std::mem::transmute([self, self]) };

        let cipher = Aes128::new(&key);

        cipher.encrypt_block(&mut block);

        unsafe { std::mem::transmute(block) }
    }
}
