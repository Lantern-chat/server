use std::{
    sync::atomic::{AtomicU16, AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use std::num::NonZeroU64;

use aes::{cipher::Key, Aes128};

pub use sdk::models::sf::{Snowflake, LANTERN_EPOCH};
use sdk::models::{Timestamp, LANTERN_EPOCH_PDT};

/// Incremenent counter to ensure unique snowflakes
pub static INCR: AtomicU16 = AtomicU16::new(0);

pub static TIME: AtomicU64 = AtomicU64::new(0);

/// Global instance value
pub static mut INST: u16 = 0;
/// Global worker value
pub static mut WORK: u16 = 0;

pub trait SnowflakeExt {
    #[inline]
    fn from_i64(id: i64) -> Option<Snowflake> {
        NonZeroU64::new(id as u64).map(Snowflake)
    }

    #[inline]
    fn max_value() -> Snowflake {
        // max signed 64-bit value, what Postgres uses for bigint
        Snowflake(unsafe { NonZeroU64::new_unchecked(i64::MAX as u64) })
    }

    // Constructs a Snowflake from the given timestamp with any of the deduplication
    // values. This is ideal for database searches using simple operators.
    fn timestamp_only(ts: SystemTime) -> Snowflake {
        let elapsed: Duration = ts.duration_since(UNIX_EPOCH).unwrap();
        let ms = elapsed.as_millis() as u64 - LANTERN_EPOCH;

        Snowflake(unsafe { NonZeroU64::new_unchecked(ms << 22) })
    }

    /// **WARNING**: DO NOT USE FOR UNIQUE IDS
    #[inline]
    fn at_unix_ms(ms: u64) -> Snowflake {
        // offset by Lantern epoch
        Self::from_parts(ms - LANTERN_EPOCH, 0)
    }

    /// **WARNING**: DO NOT USE FOR UNIQUE IDS
    #[inline]
    fn at_ts(ts: Timestamp) -> Snowflake {
        Self::at_unix_ms(ts.duration_since(Timestamp::UNIX_EPOCH).whole_milliseconds().max(0) as u64)
    }

    /// **WARNING**: DO NOT USE FOR UNIQUE IDS
    #[inline]
    fn at(ts: SystemTime) -> Snowflake {
        Self::at_unix_ms(ts.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64)
    }

    /// **WARNING**: DO NOT USE FOR UNIQUE IDS
    #[inline]
    fn at_date(ts: time::Date) -> Snowflake {
        let seconds = ts - LANTERN_EPOCH_PDT.date();

        Self::from_parts(seconds.whole_seconds() as u64 * 1000, 0)
    }

    #[inline(always)]
    fn from_parts(ms: u64, incr: u16) -> Snowflake {
        // get global IDs
        let inst = unsafe { INST as u64 };
        let worker = unsafe { WORK as u64 };

        // check inst and worker only use 5 bits
        debug_assert!(inst < (1 << 6));
        debug_assert!(worker < (1 << 6));

        // Shift into position and bitwise-OR everything together
        Snowflake(unsafe { NonZeroU64::new_unchecked((ms << 22) | (worker << 17) | (inst << 12) | (incr as u64)) })
    }

    /// Creates a new Snowflake at this moment
    fn now() -> Snowflake {
        let ms = UNIX_EPOCH.elapsed().expect("Could not get time").as_millis() as u64;

        let incr =
            INCR.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |incr| Some((incr + 1) & 0xFFF)).unwrap();

        let max_ms = TIME.fetch_max(ms, Ordering::SeqCst);

        // clock went backwards and incr is/was at max
        if incr == 0xFFF && max_ms > ms {
            // forcibly increment the timestamp until clock flows normally again
            let _ = TIME.compare_exchange(max_ms, max_ms + 1, Ordering::SeqCst, Ordering::Acquire);

            // TODO: Maybe add a log entry for this
        }

        Snowflake::from_parts(max_ms - LANTERN_EPOCH, incr)
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

    fn low_complexity(self) -> u64;
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

        const NON_TIMESTAMP_MASK: u64 = u64::MAX >> 44; // shift in zeroes from above so only timestamp bits are 0

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

    fn low_complexity(self) -> u64 {
        const ID_MASK: u64 = 0b11_1111_1111; // 10 bits
        let raw = self.to_u64();

        // shift high bits of timestamp down, since the timestamp occupies the top 42 bits,
        // shifting it down by 42 will leave only the high bits
        let ts_high = raw >> 42;
        // shift IDs down to lsb and mask them out
        let ids = (raw >> 12) & ID_MASK;
        // combine 22 timestamp bits with 10 id bits, placing the IDs first
        let high = ts_high | (ids << 22);
        // to get the low timestamp bits, shift out high bits,
        // then shift back down, then shift down again to lsb
        let ts_low = (raw << 22) >> 44;

        // to get low bits, shift timestamp over to make room for increment counter, then OR with counter
        let low = (ts_low << 12) | (raw & 0xFFF);

        // recombine
        (high << 32) | low
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_snowflake_ser() {
        assert!(serde_json::to_string(&Snowflake::now()).unwrap().contains('"'));
    }
}
