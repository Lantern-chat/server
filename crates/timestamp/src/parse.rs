use std::time::Duration;

use time::{Date, Month, PrimitiveDateTime, Time};

/// Trait implemented locally for very fast parsing of small unsigned integers
trait FastParse: Sized {
    fn parse(s: &[u8]) -> Option<Self>;
}

#[inline]
fn parse_2(s: &[u8]) -> u16 {
    let zero: u16 = 0x3030;

    let mut buf = [0; 2];
    buf.copy_from_slice(s);

    let digits = u16::from_le_bytes(buf).wrapping_sub(zero);

    //println!("DIGITS: {:04X}", digits);

    ((digits & 0x0f00) >> 8) + ((digits & 0x0f) * 10)
}

#[inline]
fn parse_4(s: &[u8]) -> u16 {
    let zero: u32 = 0x30303030;

    let mut buf = [0; 4];
    buf.copy_from_slice(s);

    let mut digits = u32::from_le_bytes(buf).wrapping_sub(zero);
    digits = ((digits & 0x0f000f00) >> 8) + ((digits & 0x000f000f) * 10);
    digits = ((digits & 0x00ff00ff) >> 16) + ((digits & 0x000000ff) * 100);
    digits as u16
}

#[inline]
fn parse_3(s: &[u8]) -> u16 {
    let mut buf = [b'0'; 4];
    buf[1..4].copy_from_slice(s);

    parse_4(&buf)
}

// TODO: Parse 5 and 6

macro_rules! impl_fp {
    ($($t:ty),*) => {$(
        impl FastParse for $t {
            #[inline]
            fn parse(s: &[u8]) -> Option<Self> {
                match s.len() {
                    2 => return Some(parse_2(s) as $t),
                    4 => return Some(parse_4(s) as $t),
                    3 => return Some(parse_3(s) as $t),
                    //1 => return Some((s[0].wrapping_sub(b'0')) as $t),
                    _ => {}
                }


                let mut num = 0;
                let mut overflow = false;

                for byte in s {
                    let digit = byte.wrapping_sub(b'0');
                    overflow |= digit > 9;
                    num = (num * 10) + digit as $t;
                }

                match overflow {
                    false => Some(num),
                    true => None,
                }
            }
        }
    )*};
}

impl_fp!(u8, u16, u32);

pub fn parse_iso8061(ts: &str) -> Option<PrimitiveDateTime> {
    let b = ts.as_bytes();

    #[inline]
    fn parse_offset<T: FastParse>(b: &[u8], offset: usize, len: usize) -> Option<T> {
        b.get(offset..(offset + len)).and_then(|x| T::parse(x))
    }

    fn is_byte(b: &[u8], offset: usize, byte: u8) -> usize {
        offset + (b.get(offset).copied() == Some(byte)) as usize
    }

    let mut offset = 0;

    let year = parse_offset::<u16>(b, offset, 4)?;
    offset = is_byte(b, offset + 4, b'-'); // YYYY-?

    //println!("YEAR: {}", year);

    let month = parse_offset::<u8>(b, offset, 2)?;
    offset = is_byte(b, offset + 2, b'-'); // MM-?

    //println!("MONTH: {}", month);

    let day = parse_offset::<u8>(b, offset, 2)?;
    offset += 2; // DD

    //println!("DAY: {}", day);

    // only parsed 4 digits
    if year > 9999 {
        unsafe { std::hint::unreachable_unchecked() }
    }

    let ymd = Date::from_calendar_date(year as i32, Month::try_from(month).ok()?, day).ok()?;

    //println!("{}-{}-{}", year, month, day);

    // if no T, then return
    if b.get(offset).map(|c| *c | 32) != Some(b't') {
        return None;
    }

    offset += 1; // T

    let hour = parse_offset::<u8>(b, offset, 2)?;
    offset = is_byte(b, offset + 2, b':');

    //println!("HOUR: {}", hour);

    let minute = parse_offset::<u8>(b, offset, 2)?;
    offset = is_byte(b, offset + 2, b':');

    //println!("MINUTE: {}", minute);

    let maybe_time;

    // if the next character is a digit, parse seconds and milliseconds, otherwise move on
    match b.get(offset) {
        Some(b'0'..=b'9') => {
            let second = parse_offset::<u8>(b, offset, 2)?;
            offset += 2;

            if b.get(offset).copied() == Some(b'.') {
                offset += 1;

                let mut factor: u32 = 1_000_000_000; // up to 9 decimal places
                let mut nanosecond: u32 = 0;

                while let Some(c) = b.get(offset) {
                    let d = c.wrapping_sub(b'0');

                    if d > 9 {
                        break; // break on non-numeric input
                    } else if factor == 0 {
                        // even if we're at nanoseconds, skip any additional digits
                        continue;
                    }

                    nanosecond = (nanosecond * 10) + d as u32;

                    offset += 1;
                    factor /= 10;
                }

                maybe_time = Time::from_hms_nano(hour, minute, second, factor * nanosecond)
            } else {
                maybe_time = Time::from_hms(hour, minute, second)
            }
        }
        _ => maybe_time = Time::from_hms(hour, minute, 0),
    }

    //println!("SECOND: {}", second);

    let mut date_time = PrimitiveDateTime::new(
        ymd,
        match maybe_time {
            Ok(time) => time,
            _ => return None,
        },
    );

    let tz = b.get(offset);

    offset += 1;

    match tz.copied() {
        // Z
        Some(b'z' | b'Z') => {}

        // timezone, like +00:00
        Some(c @ b'+' | c @ b'-' | c @ 0xe2) => {
            if c == 0xe2 {
                // check for UTF8 Unicode MINUS SIGN
                if b.get(offset..(offset + 2)) == Some(&[0x88, 0x92]) {
                    offset += 2;
                } else {
                    return None;
                }
            }

            let offset_hour = parse_offset::<u8>(b, offset, 2)? as u64;
            offset = is_byte(b, offset + 2, b':');
            let offset_minute = parse_offset::<u8>(b, offset, 2)? as u64;
            offset += 2;

            let dur = Duration::from_secs(60 * 60 * offset_hour + offset_minute * 60);

            if c == b'+' {
                date_time += dur;
            } else {
                date_time -= dur;
            }
        }

        // Parse trailing "UTC", but it does nothing, same as Z
        Some(b'U' | b'u') => match b.get(offset..(offset + 2)) {
            None => return None,
            Some(tc) => {
                for (c, r) in tc.iter().zip(b"tc") {
                    if (*c | 32) != *r {
                        return None;
                    }
                }

                offset += 2;
            }
        },
        _ => return None,
    }

    if offset != b.len() {
        return None;
    }

    Some(date_time)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_int() {
        let i = u32::parse(b"1234567890");

        assert_eq!(i, Some(1234567890));
    }

    #[test]
    fn test_parse_int2() {
        let res = parse_2(b"12");

        assert_eq!(res, 12);
    }

    #[test]
    fn test_parse_int4() {
        let res = parse_4(b"1234");

        assert_eq!(res, 1234);
    }
}
