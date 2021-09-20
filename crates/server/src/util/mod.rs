pub mod encrypt;
pub mod time;

pub fn passthrough<F, T, U>(f: F) -> F
where
    F: for<'a> FnMut(&'a T) -> &'a U,
{
    f
}
