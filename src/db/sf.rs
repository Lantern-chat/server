use std::sync::atomic::{AtomicU16, Ordering};

use std::num::NonZeroU64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Snowflake(NonZeroU64);

/// Arbitrarily chosen starting epoch to offset the clock by
pub const LANTERN_EPOCH: u64 = 1608198072352;

/// Incremenent counter to ensure unique snowflakes
pub static INCR: AtomicU16 = AtomicU16::new(0);
/// Global instance value
pub static mut INST: u16 = 0;
/// Global worker value
pub static mut WORK: u16 = 0;

impl Snowflake {
    pub fn null() -> Snowflake {
        Snowflake(unsafe { NonZeroU64::new_unchecked(1) })
    }

    /// Creates a new Snowflake at this moment
    pub fn now() -> Snowflake {
        // get time since unix epoch as milliseconds, subtract Lantern epoch
        let ms = std::time::UNIX_EPOCH
            .elapsed()
            .expect("Could not get time")
            .as_millis() as u64
            - LANTERN_EPOCH;

        // update incremenent counter, making sure it wraps at 12 bits
        let incr = INCR
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |incr| {
                Some((incr + 1) & 0x3FF)
            })
            .unwrap() as u64;

        // get global IDs
        let inst = unsafe { INST as u64 };
        let worker = unsafe { WORK as u64 };

        // check inst and worker only use 5 bits
        debug_assert!(inst < (1 << 6));
        debug_assert!(worker < (1 << 6));

        // Shift into position and bitwise-OR everything together
        Snowflake(unsafe {
            NonZeroU64::new_unchecked((ms << 22) | (worker << 17) | (inst << 12) | incr)
        })
    }

    /// Gets the number of milliseconds since the unix epoch
    pub fn epoch_ms(&self) -> u64 {
        (self.0.get() >> 22) + LANTERN_EPOCH
    }
}

use std::error::Error;

use bytes::BytesMut;
use tokio_postgres::types::{FromSql, IsNull, ToSql, Type};

impl<'a> FromSql<'a> for Snowflake {
    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        <i64 as FromSql<'a>>::from_sql(ty, raw)
            .map(|raw| Snowflake(NonZeroU64::new(raw as u64).unwrap()))
    }

    fn accepts(ty: &Type) -> bool {
        <i64 as FromSql<'a>>::accepts(ty)
    }
}

impl ToSql for Snowflake {
    fn to_sql(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>>
    where
        Self: Sized,
    {
        let value = self.0.get() as i64;
        value.to_sql(ty, out)
    }

    fn accepts(ty: &Type) -> bool
    where
        Self: Sized,
    {
        <i64 as ToSql>::accepts(ty)
    }

    fn to_sql_checked(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        let value = self.0.get() as i64;
        value.to_sql_checked(ty, out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sf_size() {
        use std::mem::size_of;
        assert_eq!(size_of::<u64>(), size_of::<Option<Snowflake>>());
    }
}
