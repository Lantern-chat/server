use time::PrimitiveDateTime;

use smol_str::SmolStr;

use itoa::Buffer;

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

pub fn format_iso8061(ts: PrimitiveDateTime) -> SmolStr {
    let (date, time) = (ts.date(), ts.time());

    let (year, month, day) = date.as_ymd();
    let (hour, minute, second, milliseconds) =
        (time.hour(), time.minute(), time.second(), time.millisecond());

    let mut pos = 0;

    //  mut buf: [u8; 20] = *b"YYYYMMDDTHHmmss.SSSZ";
    let mut buf: [u8; 20] = *b"00000000T000000.000Z";

    macro_rules! write_num {
        ($s: expr, $len: expr) => {{
            // NOTE: This likely is coalesced with the += 1's below
            pos += $len; // skip to end, then go back in the ptr below to pad with zeroes

            let mut num = Buffer::new();
            let s = num.format($s);

            unsafe {
                buf.as_mut_ptr()
                    .add(pos - s.len())
                    .copy_from_nonoverlapping(s.as_ptr(), s.len())
            }
        }};
    }

    write_num!(year, 4);
    write_num!(month, 2);
    write_num!(day, 2);
    pos += 1; // T
    write_num!(hour, 2);
    write_num!(minute, 2);
    write_num!(second, 2);
    pos += 1; // .
    write_num!(milliseconds, 3);

    SmolStr::new_inline(unsafe { std::str::from_utf8_unchecked(&buf) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(deprecated)]
    #[test]
    fn test_format_iso8061() {
        let now = PrimitiveDateTime::now();

        let formatted = format_iso8061(now);

        println!("{}", formatted);
    }
}
