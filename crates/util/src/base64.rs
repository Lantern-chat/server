use smol_str::SmolStr;

use base64::engine::{general_purpose::URL_SAFE_NO_PAD, Engine};

pub fn encode_u128(input: u128) -> SmolStr {
    let mut buf = [0u8; 22];
    URL_SAFE_NO_PAD
        .encode_slice(input.to_be_bytes(), &mut buf)
        .expect("Unable to encode u128 to base64");
    SmolStr::new_inline(unsafe { std::str::from_utf8_unchecked(&buf) })
}

pub fn decode_u128(input: &str) -> Result<u128, base64::DecodeError> {
    let mut buf = [0u8; 16];
    // NOTE: decode_slice will error on exact-size input...
    // https://github.com/marshallpierce/rust-base64/issues/210
    match URL_SAFE_NO_PAD.decode_slice_unchecked(input, &mut buf) {
        Ok(_) => Ok(u128::from_be_bytes(buf)),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64() {
        let input = 119324240026741659787093958279368883115u128;
        let x = encode_u128(input);
        let y = decode_u128(&x);

        assert_eq!(y, Ok(input));
    }
}
