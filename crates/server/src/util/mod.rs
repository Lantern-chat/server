pub mod base62;
pub mod encrypt;
pub mod serde;
pub mod time;
pub mod base64;

pub fn passthrough<F, T, U>(f: F) -> F
where
    F: for<'a> FnMut(&'a T) -> &'a U,
{
    f
}
