use smol_str::SmolStr;

pub fn decode_str(input: &str) -> Result<Result<SmolStr, std::str::Utf8Error>, base64::DecodeError> {
    match base64::decode(input) {
        Ok(v) => Ok(std::str::from_utf8(&v).map(SmolStr::new)),
        Err(e) => Err(e),
    }
}
