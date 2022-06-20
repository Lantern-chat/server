#[inline(always)]
pub fn passthrough<F, T, U>(f: F) -> F
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

impl<T: TupleClone> TupleClone for &T {
    type Output = <T as TupleClone>::Output;

    fn clone_tuple(&self) -> Self::Output {
        (**self).clone_tuple()
    }
}
