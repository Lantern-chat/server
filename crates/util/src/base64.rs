use smol_str::SmolStr;

pub fn decode_str(input: &str) -> Result<Result<SmolStr, std::str::Utf8Error>, base64::DecodeError> {
    match base64::decode(input) {
        Ok(v) => Ok(std::str::from_utf8(&v).map(SmolStr::new)),
        Err(e) => Err(e),
    }
}

pub fn encode_u128(input: u128) -> SmolStr {
    let mut buf = [0u8; 22];
    base64::encode_config_slice(&input.to_be_bytes(), base64::URL_SAFE_NO_PAD, &mut buf);
    SmolStr::new_inline(unsafe { std::str::from_utf8_unchecked(&buf) })
}

pub fn decode_u128(input: &str) -> Result<u128, base64::DecodeError> {
    let mut buf = [0u8; 16];
    match base64::decode_config_slice(input.as_bytes(), base64::URL_SAFE_NO_PAD, &mut buf) {
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
