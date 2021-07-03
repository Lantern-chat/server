use std::{
    sync::atomic::{AtomicU16, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use std::num::NonZeroU64;

pub use models::sf::{Snowflake, LANTERN_EPOCH};

/// Incremenent counter to ensure unique snowflakes
pub static INCR: AtomicU16 = AtomicU16::new(0);
/// Global instance value
pub static mut INST: u16 = 0;
/// Global worker value
pub static mut WORK: u16 = 0;

pub trait SnowflakeExt {
    #[inline]
    fn from_i64(id: i64) -> Option<Snowflake> {
        NonZeroU64::new(id as u64).map(Snowflake)
    }

    fn max_value() -> Snowflake {
        Snowflake(unsafe { NonZeroU64::new_unchecked(9223372036854775807) })
    }

    // Constructs a Snowflake from the given timestamp with any of the deduplication
    // values. This is ideal for database searches using simple operators.
    fn timestamp_only(ts: SystemTime) -> Snowflake {
        let elapsed: Duration = ts.duration_since(UNIX_EPOCH).unwrap();
        let ms = elapsed.as_millis() as u64 - LANTERN_EPOCH;

        Snowflake(unsafe { NonZeroU64::new_unchecked(ms << 22) })
    }

    /// Create a snowflake at the given unix epoch (milliseconds)
    fn at_ms(ms: u64) -> Snowflake {
        // offset by Lantern epoch
        let ms = (ms - LANTERN_EPOCH) as u64;

        // update incremenent counter, making sure it wraps at 12 bits
        let incr = INCR
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |incr| {
                Some((incr + 1) & 0xFFF)
            })
            .unwrap() as u64;

        // get global IDs
        let inst = unsafe { INST as u64 };
        let worker = unsafe { WORK as u64 };

        // check inst and worker only use 5 bits
        debug_assert!(inst < (1 << 6));
        debug_assert!(worker < (1 << 6));

        // Shift into position and bitwise-OR everything together
        Snowflake(unsafe { NonZeroU64::new_unchecked((ms << 22) | (worker << 17) | (inst << 12) | incr) })
    }

    /// Creates a new Snowflake at this moment
    fn now() -> Snowflake {
        Self::at_ms(UNIX_EPOCH.elapsed().expect("Could not get time").as_millis() as u64)
    }

    fn at(ts: SystemTime) -> Snowflake {
        Self::at_ms(ts.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64)
    }

    fn low_complexity(self) -> u64;
}

impl SnowflakeExt for Snowflake {
    fn low_complexity(self) -> u64 {
        const ID_MASK: u64 = 0b11111_11111;
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
        assert!(serde_json::to_string(&Snowflake::now()).unwrap().contains("\""));
    }
}
