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
    let (year, month, day) = ts.to_calendar_date();
    let (hour, minute, second, milliseconds) = ts.as_hms_milli();

    let mut buf = S::template();

    let mut pos = 0;

    macro_rules! write_num {
        ($s: expr, $len: expr, $max: expr) => {{
            pos += $len;
            unsafe { $s.write(buf.as_mut_ptr().add(pos), $len) }
            if S::IS_FULL { pos += 1; }
        }};
    }

    write_num!(year as u16, 4, 9999);
    write_num!(month as u8, 2, 12);
    write_num!(day, 2, 31);
    if !S::IS_FULL { pos += 1; } // T
    write_num!(hour, 2, 59);
    write_num!(minute, 2, 59);
    write_num!(second, 2, 59);
    if !S::IS_FULL { pos += 1; } // .
    write_num!(milliseconds, 3, 999);

    for (dst, offset) in buf.as_mut_slice().iter_mut().zip(S::offset().as_slice()) {
        *dst += *offset;
    }

    TimestampStr(buf)
}
