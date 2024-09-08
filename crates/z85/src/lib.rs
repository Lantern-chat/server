// Based on rustc_serialize::base64
// and also ZeroMQ's reference implementation

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

use core::fmt::{self, Error as FmtError, Write};

const CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.-:+=^!/*?&<>()[]{}@%$#";

const BYTE_OFFSETS: &[i8] = &[
    -0x01, 0x44, -0x01, 0x54, 0x53, 0x52, 0x48, -0x01, 0x4B, 0x4C, 0x46, 0x41, -0x01, 0x3F, 0x3E, 0x45, 0x00,
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x40, -0x01, 0x49, 0x42, 0x4A, 0x47, 0x51, 0x24, 0x25,
    0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2E, 0x2F, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37,
    0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x4D, -0x01, 0x4E, 0x43, -0x01, -0x01, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F, 0x20, 0x21,
    0x22, 0x23, 0x4F, -0x01, 0x50, -0x01, -0x01,
];

#[derive(Debug, Clone, Copy)]
pub enum FromZ85Error {
    /// The input contained a character not part of the Z85 format
    InvalidZ85Byte(u8, usize),
    /// The input had an invalid length
    InvalidZ85Length(usize),
}

#[derive(Debug, Clone, Copy)]
pub enum ToZ85Error {
    /// The input had an invalid length
    InvalidZ85InputSize(usize),

    /// An error occurred while writing the output
    FmtError(FmtError),
}

impl From<FmtError> for ToZ85Error {
    fn from(e: FmtError) -> Self {
        ToZ85Error::FmtError(e)
    }
}

impl fmt::Display for FromZ85Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FromZ85Error::InvalidZ85Byte(ch, idx) => {
                write!(f, "Invalid character '0x{ch:x}' at position {idx}.")
            }
            FromZ85Error::InvalidZ85Length(len) => write!(f, "Invalid length {len}."),
        }
    }
}

impl fmt::Display for ToZ85Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ToZ85Error::InvalidZ85InputSize(len) => write!(f, "Invalid input size {len}."),
            ToZ85Error::FmtError(ref e) => e.fmt(f),
        }
    }
}

impl core::error::Error for FromZ85Error {}
impl core::error::Error for ToZ85Error {}

#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};

/// A trait for converting from z85 encoded values.
pub trait ParseZ85 {
    /// Converts the value of `self`, interpreted as z85 encoded data,
    /// into an owned vector of bytes, returning the vector.
    #[cfg(feature = "alloc")]
    fn parse_z85(&self) -> Result<Vec<u8>, FromZ85Error> {
        let mut out = Vec::with_capacity(self.estimate_z85_decoded_size());
        self.parse_z85_to(&mut out)?;
        Ok(out)
    }

    fn parse_z85_to<W: Extend<u8>>(&self, writer: &mut W) -> Result<(), FromZ85Error>;

    fn estimate_z85_decoded_size(&self) -> usize;
}

/// A trait for converting a value to z85 encoding.
pub trait ToZ85 {
    /// Converts the value of `self` into a z85 encoded string,
    /// returning the owned string.
    #[cfg(feature = "alloc")]
    fn to_z85(&self) -> Result<String, ToZ85Error> {
        let mut out = String::with_capacity(self.estimate_z85_encoded_size());
        self.to_z85_in(&mut out)?;
        Ok(out)
    }

    fn to_z85_in<W: Write>(&self, writer: W) -> Result<(), ToZ85Error>;

    fn estimate_z85_encoded_size(&self) -> usize;
}

impl ParseZ85 for str {
    fn parse_z85_to<W: Extend<u8>>(&self, writer: &mut W) -> Result<(), FromZ85Error> {
        self.as_bytes().parse_z85_to(writer)
    }

    fn estimate_z85_decoded_size(&self) -> usize {
        self.as_bytes().estimate_z85_decoded_size()
    }
}

impl ParseZ85 for [u8] {
    fn parse_z85_to<W: Extend<u8>>(&self, writer: &mut W) -> Result<(), FromZ85Error> {
        let len = self.len();
        if len == 0 || len % 5 != 0 {
            return Err(FromZ85Error::InvalidZ85Length(len));
        }

        let mut pos: usize = 0;
        while pos < len {
            let mut block_num: u32 = 0;
            let next_pos = pos + 5;
            for c in &self[pos..next_pos] {
                if *c <= 32 || *c > 127 {
                    return Err(FromZ85Error::InvalidZ85Byte(*c, pos));
                }
                let kar = BYTE_OFFSETS[(*c as usize) - 32];
                if kar == -1 {
                    return Err(FromZ85Error::InvalidZ85Byte(*c, pos));
                }
                block_num = block_num * 85 + kar as u32;
            }
            // reverse block_num bytes
            writer.extend(block_num.swap_bytes().to_ne_bytes());
            pos = next_pos;
        }

        Ok(())
    }

    fn estimate_z85_decoded_size(&self) -> usize {
        self.len() / 5 * 4
    }
}

impl ToZ85 for [u8] {
    fn estimate_z85_encoded_size(&self) -> usize {
        self.len() * 5 / 4
    }

    /// Turn a vector of `u8` bytes into a base64 string.
    fn to_z85_in<W: Write>(&self, mut writer: W) -> Result<(), ToZ85Error> {
        let len = self.len();

        if len == 0 || len % 4 != 0 {
            return Err(ToZ85Error::InvalidZ85InputSize(len));
        }

        for in_chunk in self.chunks(4) {
            let mut block_num: u32 = 0;
            for byte in in_chunk {
                block_num = (block_num << 8) | (*byte as u32);
            }
            let mut out_chunk = [0_u8; 5];
            for c in out_chunk.as_mut_slice() {
                *c = CHARS[(block_num % 85) as usize];
                block_num /= 85;
            }
            out_chunk.reverse();
            writer.write_str(unsafe { core::str::from_utf8_unchecked(&out_chunk) })?;
        }

        Ok(())
    }
}
