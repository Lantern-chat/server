use smol_str::SmolStr;
use util::hex::HexidecimalInt;

#[inline(never)]
#[no_mangle]
pub fn test64_to_hex(x: u64) -> SmolStr {
    HexidecimalInt(x).to_hex()
}

#[inline(never)]
#[no_mangle]
pub fn test64_to_string(x: u64) -> String {
    HexidecimalInt(x).to_string()
}

//#[inline(never)]
//#[no_mangle]
//pub fn test_b62_128(x: u128) -> SmolStr {
//    util::base62::encode128(x)
//}

#[inline(never)]
#[no_mangle]
pub fn test128_to_hex(x: u128) -> SmolStr {
    HexidecimalInt(x).to_hex()
}

#[inline(never)]
#[no_mangle]
pub fn test128_to_b64(x: u128) -> SmolStr {
    util::base64::encode_u128(x)
}

#[inline(never)]
#[no_mangle]
pub fn format_iso8061(ts: time::PrimitiveDateTime) -> SmolStr {
    util::time::format_iso8061(ts)
}

fn main() {}
