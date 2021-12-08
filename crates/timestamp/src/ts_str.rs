use generic_array::{ArrayLength, GenericArray};

mod sealed {
    pub trait Sealed {}
}

#[doc(hidden)]
pub trait TimestampStrStorage: sealed::Sealed {
    type Length: ArrayLength<u8>;

    fn init() -> GenericArray<u8, Self::Length>;

    const IS_FULL: bool;
}

pub struct Short;
pub struct Full;

impl sealed::Sealed for Short {}
impl sealed::Sealed for Full {}

impl TimestampStrStorage for Short {
    type Length = generic_array::typenum::consts::U20;

    #[inline(always)]
    fn init() -> GenericArray<u8, Self::Length> {
        //nericArray::from(*b"YYYYMMDDTHHmmss.SSSZ")
        GenericArray::from(*b"00000000T000000.000Z")
    }

    const IS_FULL: bool = false;
}

impl TimestampStrStorage for Full {
    type Length = generic_array::typenum::consts::U24;

    #[inline(always)]
    fn init() -> GenericArray<u8, Self::Length> {
        //nericArray::from(*b"YYYY-MM-DDTHH:mm:ss.SSSZ")
        GenericArray::from(*b"0000-00-00T00:00:00.000Z")
    }

    const IS_FULL: bool = true;
}

pub struct TimestampStr<S: TimestampStrStorage>(pub(crate) GenericArray<u8, S::Length>);

impl<S: TimestampStrStorage> AsRef<str> for TimestampStr<S> {
    #[inline]
    fn as_ref(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }
}

use std::borrow::Borrow;

impl<S: TimestampStrStorage> Borrow<str> for TimestampStr<S> {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_ref()
    }
}

use std::ops::Deref;

impl<S: TimestampStrStorage> Deref for TimestampStr<S> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<S: TimestampStrStorage> PartialEq for TimestampStr<S> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl<S: TimestampStrStorage> PartialEq<str> for TimestampStr<S> {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_ref() == other
    }
}

impl<S: TimestampStrStorage> PartialEq<TimestampStr<S>> for str {
    #[inline]
    fn eq(&self, other: &TimestampStr<S>) -> bool {
        self == other.as_ref()
    }
}

use std::fmt;

impl<S: TimestampStrStorage> fmt::Debug for TimestampStr<S> {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_ref(), f)
    }
}

impl<S: TimestampStrStorage> fmt::Display for TimestampStr<S> {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_ref(), f)
    }
}
