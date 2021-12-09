use time::PrimitiveDateTime;

use crate::ts_str::{TimestampStr, TimestampStrStorage};

const fn make_table() -> [[u8; 2]; 100] {
    let mut table = [[0; 2]; 100];

    let mut i: u8 = 0;
    while i < 10 {
        let mut j: u8 = 0;
        while j < 10 {
            table[(i as usize) * 10 + (j as usize)] = [i + b'0', j + b'0'];
            j += 1;
        }
        i += 1;
    }

    table
}

const LOOKUP: [[u8; 2]; 100] = make_table();

#[rustfmt::skip]
#[allow(unused_assignments)]
#[inline(always)]
pub fn format_iso8061<S: TimestampStrStorage>(ts: PrimitiveDateTime) -> TimestampStr<S> {
    // decompose timestamp
    let (year, month, day) = ts.to_calendar_date();
    let (hour, minute, second, milliseconds) = ts.as_hms_milli();

    let mut buf = S::init();
    let mut pos = 0;

    macro_rules! write_num {
        ($s: expr, $len: expr, $max: expr) => {unsafe {
            let value = $s;

            // tell the compiler that the max value is known
            if value > $max { std::hint::unreachable_unchecked() }

            let buf = buf.as_mut_ptr().add(pos);
            let lookup = LOOKUP.as_ptr();

            match $len {
                2 => {
                    buf.copy_from_nonoverlapping(lookup.add(value as usize) as *const u8, 2);
                }
                3 => {
                    let ab = value / 10;
                    let c = value % 10;

                    buf.copy_from_nonoverlapping(lookup.add(ab as usize) as *const u8, 2);
                    //*buf.add(2) = c + b'0';
                    *buf.add(2) = (*lookup.add(c as usize))[1];
                }
                4 => {
                    let value = value as u16;

                    let ab = value / 100;
                    let cd = value % 100;

                    buf.copy_from_nonoverlapping(lookup.add(ab as usize) as *const u8, 2);
                    buf.add(2).copy_from_nonoverlapping(lookup.add(cd as usize) as *const u8, 2);
                }
                _ => std::hint::unreachable_unchecked()
            }

            pos += $len;

            if S::IS_FULL { pos += 1; }
        }};
    }

    write_num!(year as u16,     4, 9999);   // YYYY-
    write_num!(month as u8,     2, 12);     // MM-
    write_num!(day,             2, 31);     // DDT?
    if !S::IS_FULL { pos += 1; }            // T
    write_num!(hour,            2, 59);     // HH:
    write_num!(minute,          2, 59);     // mm:
    write_num!(second,          2, 59);     // ss.?
    if !S::IS_FULL { pos += 1; }            // .
    write_num!(milliseconds,    3, 999);    // SSS

    TimestampStr(buf)
}
