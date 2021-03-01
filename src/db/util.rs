#[inline]
pub const fn is_false(value: &bool) -> bool {
    !*value
}
