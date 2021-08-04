use std::fmt::{self, Display, Formatter, LowerHex};
use std::str::FromStr;

/// Integer wrapper that can `FromStr` and `Display` hexidecimal values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct HexidecimalInt<T>(pub T);

pub unsafe trait SafeInt: Sized + LowerHex {
    type BYTES: Default + AsRef<[u8]> + AsMut<[u8]>;
    fn from_bytes(bytes: Self::BYTES) -> Self;
    fn into_bytes(self) -> Self::BYTES;
}

macro_rules! impl_safe_int {
    ($($ty:ty: $bytes:expr),*) => {$(
        unsafe impl SafeInt for $ty {
            type BYTES = [u8; $bytes];

            #[inline(always)]
            fn from_bytes(bytes: Self::BYTES) -> Self {
                unsafe { std::mem::transmute(bytes) }
            }

            #[inline(always)]
            fn into_bytes(self) -> Self::BYTES {
                unsafe { std::mem::transmute(self) }
            }
        }
    )*}
}

impl_safe_int!(i8: 1, i16: 2, i32: 4, i64: 8, i128: 16, u8: 1, u16: 2, u32: 4, u64: 8, u128: 16);

impl<T: SafeInt> FromStr for HexidecimalInt<T> {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut bytes = T::BYTES::default();
        hex::decode_to_slice(s, bytes.as_mut())?;
        Ok(HexidecimalInt(T::from_bytes(bytes)))
    }
}

impl<T: LowerHex> Display for HexidecimalInt<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:01$x}", self.0, std::mem::size_of::<Self>() * 2)
    }
}
