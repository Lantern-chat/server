use std::fmt::{self, Debug, Display, Formatter, LowerHex};
use std::str::FromStr;

use smol_str::SmolStr;

/// Integer wrapper that can `FromStr` and `Display` hexidecimal values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct HexidecimalInt<T>(pub T);

pub unsafe trait SafeInt: Sized + LowerHex + Copy {
    type Bytes: Default + AsRef<[u8]> + AsMut<[u8]> + Debug;
    type HexBytes: Default + AsRef<[u8]> + AsMut<[u8]> + Debug;

    fn __from_be_bytes(bytes: Self::Bytes) -> Self;
    fn __to_be_bytes(self) -> Self::Bytes;
}

macro_rules! impl_safe_int {
    ($($ty:ty: $bytes:expr),*) => {$(
        unsafe impl SafeInt for $ty {
            type Bytes = [u8; $bytes];
            type HexBytes = [u8; $bytes * 2];

            #[inline(always)]
            fn __from_be_bytes(bytes: Self::Bytes) -> Self {
                <$ty>::from_be_bytes(bytes)
            }

            #[inline(always)]
            fn __to_be_bytes(self) -> Self::Bytes {
                self.to_be_bytes()
            }
        }
    )*}
}

impl_safe_int!(i8: 1, i16: 2, i32: 4, i64: 8, i128: 16, u8: 1, u16: 2, u32: 4, u64: 8, u128: 16);

impl<T: SafeInt> FromStr for HexidecimalInt<T> {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut bytes = T::Bytes::default();
        // NOTE: This decodes as Big Endian
        hex::decode_to_slice(s, bytes.as_mut())?;
        Ok(HexidecimalInt(T::__from_be_bytes(bytes)))
    }
}

impl<T: SafeInt> Display for HexidecimalInt<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:01$x}", self.0, std::mem::size_of::<Self>() * 2)
    }
}

impl<T: SafeInt> HexidecimalInt<T> {
    pub fn to_hex(&self) -> SmolStr {
        let bytes = self.0.__to_be_bytes();
        let mut buf = T::HexBytes::default();

        let s = match hex::encode_to_slice(bytes.as_ref(), buf.as_mut()) {
            Err(_) => unsafe { std::hint::unreachable_unchecked() },
            Ok(_) => unsafe { std::str::from_utf8_unchecked(buf.as_ref()) },
        };

        // size * 2 <= 22, max inline size of SmolStr
        if std::mem::size_of::<Self>() <= 11 {
            SmolStr::new_inline(s)
        } else {
            SmolStr::new(s)
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_hex() {
        let num: u64 = 0xFF_DEAD_BEEF_EE_AA_04;

        let h = HexidecimalInt(num);

        let a = h.to_string();
        let b = h.to_hex();

        assert_eq!(a, b);

        assert_eq!(HexidecimalInt::<u64>::from_str(a.as_str()), Ok(h));
    }
}
