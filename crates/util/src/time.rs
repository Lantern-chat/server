use std::time::Duration;

use time::{Date, PrimitiveDateTime, Time};

use smol_str::SmolStr;

//use itoa::Buffer;

/*
#[rustfmt::skip]
pub fn format_iso8061_old(ts: PrimitiveDateTime) -> SmolStr {
    use std::io::{Cursor, Write};

    let (date, time) = (ts.date(), ts.time());

    let (year, month, day) = date.as_ymd();
    let (hour, minute, second, milliseconds) =
        (time.hour(), time.minute(), time.second(), time.millisecond());

    let mut buf = [0u8; 22];
    let mut cur = Cursor::new(buf.as_mut());

    write!(cur, "{:04}{:02}{:02}T{:02}{:02}{:02}.{:03}Z",
        year, month, day, hour, minute, second, milliseconds).unwrap();

    let written = cur.position() as usize;
    SmolStr::new_inline(unsafe { std::str::from_utf8_unchecked(&buf[..written]) })
}
*/

trait FastFormat {
    unsafe fn write(self, buf: *mut u8);
}

macro_rules! impl_ff {
    ($($t:ty),*) => {$(
        impl FastFormat for $t {
            #[inline]
            unsafe fn write(mut self, mut buf: *mut u8) {
                while self > 0 {
                    buf = buf.sub(1);
                    *buf = (self % 10) as u8 + b'0';
                    self /= 10;
                }
            }
        }
    )*}
}

impl_ff!(u8, u16);

pub fn format_iso8061(ts: PrimitiveDateTime) -> SmolStr {
    let (date, time) = (ts.date(), ts.time());

    let (year, month, day) = date.as_ymd();
    let (hour, minute, second, milliseconds) =
        (time.hour(), time.minute(), time.second(), time.millisecond());

    let mut pos = 0;

    //  mut buf: [u8; 20] = *b"YYYYMMDDTHHmmss.SSSZ";
    let mut buf: [u8; 20] = *b"00000000T000000.000Z";

    macro_rules! write_num {
        ($s: expr, $len: expr, $max: expr) => {{
            // NOTE: This likely is coalesced with the += 1's below
            //pos += $len; // skip to end, then go back in the ptr below to pad with zeroes

            let value = $s;

            debug_assert!(value <= $max);
            if value > $max {
                unsafe { std::hint::unreachable_unchecked() }
            }

            pos += $len;
            unsafe { value.write(buf.as_mut_ptr().add(pos)) }
            //pos += 1;

            if pos > 19 {
                unsafe { std::hint::unreachable_unchecked() }
            }

            /*
            let mut num_buffer = Buffer::new();
            let s = num_buffer.format(value);

            unsafe {
                buf.as_mut_ptr()
                    .add(pos - s.len())
                    .copy_from_nonoverlapping(s.as_ptr(), s.len())
            }
             */
        }};
    }

    write_num!(year as u16, 4, 9999);
    write_num!(month, 2, 12);
    write_num!(day, 2, 31);
    pos += 1; // T
    write_num!(hour, 2, 59);
    write_num!(minute, 2, 59);
    write_num!(second, 2, 59);
    pos += 1; // .
    write_num!(milliseconds, 3, 999);

    debug_assert_eq!(pos, 19);

    if pos != 19 {
        unsafe { std::hint::unreachable_unchecked() }
    }

    SmolStr::new_inline(unsafe { std::str::from_utf8_unchecked(&buf) })
}

/*
use regex::{Regex, RegexBuilder};

lazy_static::lazy_static! {
    static ref ISO8061_REGEX: Regex = RegexBuilder::new(r"
        ([0-9]{4})-?    # year
        ([0-9]{2})-?    # month
        ([0-9]{2})      # day
        T
        ([0-9]{2}):?    # hour
        ([0-9]{2}):?    # minute
        ([0-9]{2})      # second
        (?:\.([0-9]+))? # milliseconds
        Z
        "
    ).ignore_whitespace(true).unicode(false).build().unwrap();

    static ref ISO8061_REGEX2: Regex = Regex::new("([0-9]{4})-?([0-9]{2})-?([0-9]{2})T([0-9]{2}):?([0-9]{2}):?([0-9]{2})(\\.[0-9]+)?Z").unwrap();
}

pub fn parse_iso8061_regex(ts: &str) -> Option<PrimitiveDateTime> {
    let m = ISO8061_REGEX.captures(ts)?;

    fn do_parse<T: FromStr>(m: Option<regex::Match>) -> Option<T> {
        //println!("Parsing {:?}", m.map(|m| m.as_str()));

        match m {
            Some(m) => m.as_str().parse().ok(),
            None => None,
        }
    }

    let year: i32 = do_parse(m.get(1))?;
    let month: u8 = do_parse(m.get(2))?;
    let day: u8 = do_parse(m.get(3))?;

    //println!("{}-{}-{}", year, month, day);

    let ymd = Date::try_from_ymd(year, month, day).ok()?;

    let hour: u8 = do_parse(m.get(4))?;
    let minute: u8 = do_parse(m.get(5))?;
    let second: u8 = do_parse(m.get(6))?;
    let millisecond: u16 = do_parse(m.get(7)).unwrap_or(0);

    //println!("{}:{}:{}.{}", hour, minute, second, millisecond);

    let t = Time::try_from_hms_milli(hour, minute, second, millisecond).ok()?;

    Some(PrimitiveDateTime::new(ymd, t))
}

pub fn parse_iso8061_old(ts: &str) -> Option<PrimitiveDateTime> {
    if ts.len() < MIN_SIZE {
        return None;
    }

    let mut offset = 0;

    fn parse_range<T: FromStr>(s: &str, offset: usize, len: usize) -> Option<T> {
        s.get(offset..(offset + len))?.parse().ok()
    }

    fn is_byte(s: &str, offset: usize, byte: u8) -> usize {
        offset + (s.as_bytes().get(offset).copied() == Some(byte)) as usize
    }

    let year: i32 = parse_range(ts, offset, 4)?;
    offset = is_byte(ts, offset + 4, b'-');

    let month: u8 = parse_range(ts, offset, 2)?;
    offset = is_byte(ts, offset + 2, b'-');

    let day: u8 = parse_range(ts, offset, 2)?;
    offset += 2;

    if offset == is_byte(ts, offset, b'T') {
        return None;
    }

    println!("{}-{}-{}", year, month, day);

    let ymd = Date::try_from_ymd(year, month, day).ok()?;

    let hour: u8 = parse_range(ts, offset + 1, 2)?;
    offset = is_byte(ts, offset + 3, b':');

    let minute: u8 = parse_range(ts, offset, 2)?;
    offset = is_byte(ts, offset + 2, b':');

    let second: u8 = parse_range(ts, offset, 2)?;
    offset += 2;

    let mut millisecond = 0;

    if offset == is_byte(ts, offset, b'.') {
        millisecond = parse_range(ts, offset + 1, 3)?;
        offset += 3;
    }

    offset = is_byte(ts, offset, b'Z');

    if offset != ts.len() {
        println!("HERE {} == {}", offset, ts.len());
        return None;
    }

    let time = Time::try_from_hms_milli(hour, minute, second, millisecond).ok()?;

    Some(PrimitiveDateTime::new(ymd, time))
}
 */

/// Trait implemented locally for very fast parsing of small unsigned integers
trait FastParse: Sized {
    fn parse(s: &[u8]) -> Option<Self>;
}

#[inline]
pub fn parse_2(s: &[u8]) -> u16 {
    let zero: u16 = 0x3030;

    let mut buf = [0; 2];
    buf.copy_from_slice(s);

    let digits = u16::from_le_bytes(buf).wrapping_sub(zero);

    //println!("DIGITS: {:04X}", digits);

    ((digits & 0x0f00) >> 8) + ((digits & 0x0f) * 10)
}

#[inline]
pub fn parse_4(s: &[u8]) -> u16 {
    let zero: u32 = 0x30303030;

    let mut buf = [0; 4];
    buf.copy_from_slice(s);

    let mut digits = u32::from_le_bytes(buf).wrapping_sub(zero);
    digits = ((digits & 0x0f000f00) >> 8) + ((digits & 0x000f000f) * 10);
    digits = ((digits & 0x00ff00ff) >> 16) + ((digits & 0x000000ff) * 100);
    digits as u16
}

#[inline]
pub fn parse_3(s: &[u8]) -> u16 {
    let mut buf = [b'0'; 4];
    buf[1..4].copy_from_slice(s);

    parse_4(&buf)
}

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

    let ymd = Date::try_from_ymd(year as i32, month, day).ok()?;

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
                let millisecond = parse_offset::<u16>(b, offset + 1, 3)?;
                offset += 4;

                // only parsed 3 digits
                if millisecond > 999 {
                    unsafe { std::hint::unreachable_unchecked() }
                }

                maybe_time = Time::try_from_hms_milli(hour, minute, second, millisecond)
            } else {
                maybe_time = Time::try_from_hms(hour, minute, second);
            }
        }
        _ => {
            maybe_time = Time::try_from_hms(hour, minute, 0);
        }
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
#[allow(deprecated)]
mod tests {
    use super::*;

    #[test]
    fn test_format_iso8061() {
        let now = PrimitiveDateTime::now();

        let formatted = format_iso8061(now);

        println!("{}", formatted);
    }

    #[test]
    fn test_parse_iso8061_reflex() {
        //println!("{}", ISO8061_REGEX.as_str());

        let now = PrimitiveDateTime::now();

        let formatted = format_iso8061(now);

        println!("Formatted: {}", formatted);

        let parsed = parse_iso8061(&formatted).unwrap();

        assert_eq!(formatted, format_iso8061(parsed));
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
            let parsed = parse_iso8061(fixture);
            assert!(parsed.is_some());

            println!("{:?}", parsed.unwrap());
        }
    }

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
