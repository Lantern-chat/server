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

pub fn parse_boolean(value: &str) -> Result<bool, std::num::ParseIntError> {
    Ok(if value.eq_ignore_ascii_case("true") {
        true
    } else if value.eq_ignore_ascii_case("false") {
        false
    } else {
        1 == u8::from_str_radix(value, 2)?
    })
}
