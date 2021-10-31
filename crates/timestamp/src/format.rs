use time::PrimitiveDateTime;

trait FastFormat {
    unsafe fn write(self, buf: *mut u8, size: usize);
}

macro_rules! impl_ff {
    ($($t:ty),*) => {$(
        impl FastFormat for $t {
            #[inline(always)]
            unsafe fn write(mut self, buf: *mut u8, mut size: usize) {
                let mut offset = 1;

                while size > 0 {
                    *buf.sub(offset) = (self % 10) as u8;
                    self /= 10;
                    offset += 1;
                    size -= 1;
                }
            }
        }
    )*}
}

impl_ff!(u8, u16);

use crate::ts_str::{TimestampStr, TimestampStrStorage};

#[rustfmt::skip]
#[allow(unused_assignments)]
#[inline(always)]
pub fn format_iso8061<S: TimestampStrStorage>(ts: PrimitiveDateTime) -> TimestampStr<S> {
    use generic_array::functional::FunctionalSequence; // for zip

    // decompose timestamp
    let (year, month, day) = ts.to_calendar_date();
    let (hour, minute, second, milliseconds) = ts.as_hms_milli();

    // initial buffer of zero values
    let mut buf = S::zero();
    let mut pos = 0;

    macro_rules! write_num {
        ($s: expr, $len: expr, $max: expr) => {{
            let value = $s;

            // tell the compiler that the max value is known
            if value > $max { unsafe { std::hint::unreachable_unchecked() } }

            pos += $len;
            unsafe { value.write(buf.as_mut_ptr().add(pos), $len) }

            if S::IS_FULL { pos += 1; } // full punctuation
        }};
    }

    write_num!(year as u16,     4, 9999);   // YYYY
    write_num!(month as u8,     2, 12);     // MM
    write_num!(day,             2, 31);     // DD
    if !S::IS_FULL { pos += 1; }            // T
    write_num!(hour,            2, 59);     // HH
    write_num!(minute,          2, 59);     // mm
    write_num!(second,          2, 59);     // ss
    if !S::IS_FULL { pos += 1; }            // .
    write_num!(milliseconds,    3, 999);    // SSS

    // offset binary values to ASCII and return
    TimestampStr(buf.zip(S::offset(), std::ops::Add::add))
}
