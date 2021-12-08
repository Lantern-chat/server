use std::ops::{Deref, DerefMut};
use std::time::{Duration, SystemTime};

use time::{OffsetDateTime, PrimitiveDateTime, UtcOffset};

#[macro_use]
mod macros;

mod format;
mod parse;
mod ts_str;

pub use ts_str::{Full, Short, TimestampStr};

/// Timestamp with Nanosecond/Millisecond precision. Nanosecond internally, Millisecond when serialized to JSON
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Timestamp(PrimitiveDateTime);

use std::fmt;

impl fmt::Debug for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ts = self.format();

        f.debug_tuple("Timestamp").field(&ts).finish()
    }
}

impl From<SystemTime> for Timestamp {
    fn from(ts: SystemTime) -> Self {
        Timestamp(match ts.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(dur) => Self::PRIMITIVE_UNIX_EPOCH + dur,
            Err(err) => Self::PRIMITIVE_UNIX_EPOCH - err.duration(),
        })
    }
}

impl From<OffsetDateTime> for Timestamp {
    fn from(ts: OffsetDateTime) -> Self {
        let utc_datetime = ts.to_offset(UtcOffset::UTC);
        let date = utc_datetime.date();
        let time = utc_datetime.time();
        Timestamp(PrimitiveDateTime::new(date, time))
    }
}

impl From<PrimitiveDateTime> for Timestamp {
    #[inline]
    fn from(ts: PrimitiveDateTime) -> Self {
        Timestamp(ts)
    }
}

impl Timestamp {
    const PRIMITIVE_UNIX_EPOCH: PrimitiveDateTime = time::macros::datetime!(1970 - 01 - 01 00:00);

    pub const UNIX_EPOCH: Self = Timestamp(Self::PRIMITIVE_UNIX_EPOCH);

    #[inline]
    pub fn now_utc() -> Self {
        SystemTime::now().into()
    }

    pub fn from_unix_timestamp(seconds: i64) -> Self {
        if seconds < 0 {
            Self::UNIX_EPOCH - Duration::from_secs(-seconds as u64)
        } else {
            Self::UNIX_EPOCH + Duration::from_secs(seconds as u64)
        }
    }

    pub fn from_unix_timestamp_ms(milliseconds: i64) -> Self {
        if milliseconds < 0 {
            Self::UNIX_EPOCH - Duration::from_millis(-milliseconds as u64)
        } else {
            Self::UNIX_EPOCH + Duration::from_millis(milliseconds as u64)
        }
    }

    pub fn to_unix_timestamp_ms(self) -> i64 {
        const UNIX_EPOCH_JULIAN_DAY: i64 = time::macros::date!(1970 - 01 - 01).to_julian_day() as i64;

        let day = self.to_julian_day() as i64 - UNIX_EPOCH_JULIAN_DAY;
        let (hour, minute, second, ms) = self.as_hms_milli();

        let hours = day * 24 + hour as i64;
        let minutes = hours * 60 + minute as i64;
        let seconds = minutes * 60 + second as i64;
        let millis = seconds * 1000 + ms as i64;

        millis
    }

    pub fn format(&self) -> TimestampStr<Full> {
        format::format_iso8061(self.0)
    }

    pub fn format_short(&self) -> TimestampStr<Short> {
        format::format_iso8061(self.0)
    }

    #[inline]
    pub fn parse(ts: &str) -> Option<Self> {
        parse::parse_iso8061(ts).map(Timestamp)
    }

    pub fn assume_offset(self, offset: time::UtcOffset) -> time::OffsetDateTime {
        self.0.assume_offset(offset)
    }
}

impl Deref for Timestamp {
    type Target = PrimitiveDateTime;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Timestamp {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> std::ops::Add<T> for Timestamp
where
    PrimitiveDateTime: std::ops::Add<T, Output = PrimitiveDateTime>,
{
    type Output = Self;

    #[inline]
    fn add(self, rhs: T) -> Self::Output {
        Timestamp(self.0 + rhs)
    }
}

impl<T> std::ops::Sub<T> for Timestamp
where
    PrimitiveDateTime: std::ops::Sub<T, Output = PrimitiveDateTime>,
{
    type Output = Self;

    #[inline]
    fn sub(self, rhs: T) -> Self::Output {
        Timestamp(self.0 - rhs)
    }
}

mod serde_impl {
    use serde::de::{Deserialize, Deserializer, Error, Visitor};
    use serde::ser::{Serialize, Serializer};

    use super::Timestamp;

    impl Serialize for Timestamp {
        #[inline]
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            if serializer.is_human_readable() {
                self.format().serialize(serializer)
            } else {
                self.to_unix_timestamp_ms().serialize(serializer)
            }
        }
    }

    impl<'de> Deserialize<'de> for Timestamp {
        #[inline]
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            use std::fmt;

            struct TsVisitor;

            impl<'de> Visitor<'de> for TsVisitor {
                type Value = Timestamp;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("an ISO8061 Timestamp")
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    match Timestamp::parse(v) {
                        Some(ts) => Ok(ts),
                        None => Err(E::custom("Invalid Format")),
                    }
                }

                fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(Timestamp::from_unix_timestamp_ms(v))
                }
            }

            deserializer.deserialize_str(TsVisitor)
        }
    }
}

mod pg_impl {
    use postgres_types::{accepts, to_sql_checked, FromSql, IsNull, ToSql, Type};
    use time::PrimitiveDateTime;

    use super::Timestamp;

    impl ToSql for Timestamp {
        #[inline]
        fn to_sql(
            &self,
            ty: &Type,
            out: &mut bytes::BytesMut,
        ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>>
        where
            Self: Sized,
        {
            self.0.to_sql(ty, out)
        }

        accepts!(TIMESTAMP, TIMESTAMPTZ);
        to_sql_checked!();
    }

    impl<'a> FromSql<'a> for Timestamp {
        #[inline]
        fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
            PrimitiveDateTime::from_sql(ty, raw).map(Timestamp)
        }

        accepts!(TIMESTAMP, TIMESTAMPTZ);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_iso8061() {
        let now = Timestamp::now_utc();

        let formatted = now.format();

        println!("{}", formatted);
    }

    #[test]
    fn test_format_iso8061_full() {
        let now = Timestamp::now_utc();

        let formatted = now.format();

        println!("{}", formatted);
    }

    #[test]
    fn test_parse_iso8061_reflex() {
        //println!("{}", Timestamp_REGEX.as_str());

        let now = Timestamp::now_utc();

        let formatted = now.format();

        println!("Formatted: {}", formatted);

        let parsed = Timestamp::parse(&formatted).unwrap();

        assert_eq!(formatted, parsed.format());
    }

    #[test]
    fn test_parse_iso8061_variations() {
        let fixtures = [
            "2021-10-17T02:03:01+00:00",
            "2021-10-17t02:03:01+10:00",
            "2021-10-17t02:03+00:00", // without seconds
            "2021-10-17t02:03:01.111+00:00",
            "2021-10-17T02:03:01-00:00",
            "2021-10-17T02:03:01−04:00", // UNICODE MINUS SIGN in offset
            "2021-10-17T02:03:01Z",
            "20211017T020301Z",
            "20211017t020301z",
            "20211017T0203z", // without seconds
            "20211017T020301.123Z",
            "20211017T020301.123+00:00",
            "20211017T020301.123uTc",
        ];

        for fixture in fixtures {
            let parsed = Timestamp::parse(fixture);

            assert!(parsed.is_some(), "Failed to parse: {}", fixture);

            println!("{:?}", parsed.unwrap());
        }
    }

    #[test]
    fn test_unix_timestamp_ms() {
        let now_ts = Timestamp::now_utc();
        let now_ot = now_ts.assume_offset(time::UtcOffset::UTC);

        let unix_ms_a = now_ts.to_unix_timestamp_ms();
        let unix_ms_b = (now_ot.unix_timestamp_nanos() / 1_000_000) as i64;

        assert_eq!(unix_ms_a, unix_ms_b);
    }

    #[test]
    fn test_parse_nanoseconds() {
        let parsed = Timestamp::parse("2021-11-19T04:12:54.000123Z").unwrap();

        let time = time::Time::from_hms_nano(4, 12, 54, 123000).unwrap();
        let date = time::Date::from_calendar_date(2021, time::Month::November, 19).unwrap();

        let expected = Timestamp::from(time::PrimitiveDateTime::new(date, time));

        assert_eq!(parsed, expected);
    }
}
