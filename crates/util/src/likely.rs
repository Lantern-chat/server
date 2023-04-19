#[inline]
#[cold]
fn cold() {}

#[rustfmt::skip]
#[inline(always)]
pub fn likely(b: bool) -> bool {
    if !b { cold() } b
}

#[rustfmt::skip]
#[inline(always)]
pub fn unlikely(b: bool) -> bool {
    if b { cold() } b
}
