use std::borrow::Cow;

#[inline]
pub fn decompress_if(cond: bool, msg: &[u8]) -> Result<Cow<[u8]>, std::io::Error> {
    if !cond {
        return Ok(Cow::Borrowed(msg));
    }

    match util::zlib::inflate(msg, Some(1024 * 1024 * 1024)) {
        Ok(decompressed) => Ok(Cow::Owned(decompressed)),
        Err(err) => Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, err)),
    }
}
