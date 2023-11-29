//pub mod base62;
pub mod base;
pub mod base64;
pub mod cmap;
pub mod hex;
pub mod laggy;
pub mod likely;
pub mod rng;
pub mod serde;
pub mod string;
pub mod time;
pub mod zlib;

pub fn parse_boolean(value: &str) -> Result<bool, std::num::ParseIntError> {
    Ok(if value.eq_ignore_ascii_case("true") {
        true
    } else if value.eq_ignore_ascii_case("false") {
        false
    } else {
        1 == u8::from_str_radix(value, 2)?
    })
}

#[inline(always)]
pub const fn passthrough<F, T, U>(f: F) -> F
where
    F: for<'a> FnMut(&'a T) -> &'a U,
{
    f
}

pub trait TupleClone {
    type Output;

    fn clone_tuple(&self) -> Self::Output;
}

macro_rules! impl_tuple_clone {
    ($(($($f:ident),*);)*) => {
        $(
            impl<$($f: Clone),*> TupleClone for ($( &$f ,)*) {
                type Output = ($($f,)*);

                #[inline]
                #[allow(non_snake_case)]
                fn clone_tuple(&self) -> Self::Output {
                    let ($($f,)*) = self;

                    ($(<$f>::clone($f),)*)
                }
            }
        )*
    }
}

impl_tuple_clone! {
    (A);
    (A, B);
    (A, B, C);
    (A, B, C, D);
    (A, B, C, D, E);
    (A, B, C, D, E, F);
    (A, B, C, D, E, F, G);
    (A, B, C, D, E, F, G, H);
    (A, B, C, D, E, F, G, H, I);
    (A, B, C, D, E, F, G, H, I, J);
    (A, B, C, D, E, F, G, H, I, J, K);
}

// TODO: Also implement for Box?
impl<T: TupleClone> TupleClone for &T {
    type Output = <T as TupleClone>::Output;

    #[inline]
    fn clone_tuple(&self) -> Self::Output {
        (**self).clone_tuple()
    }
}
