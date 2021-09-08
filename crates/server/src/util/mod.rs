pub mod base62;
pub mod hex;
pub mod serde;
pub mod time;
pub mod encrypt;

pub fn passthrough<F, T, U>(f: F) -> F
where
    F: for<'a> FnMut(&'a T) -> &'a U,
{
    f
}
