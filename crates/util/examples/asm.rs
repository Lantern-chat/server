use smol_str::SmolStr;
use util::hex::HexidecimalInt;

#[inline(never)]
#[no_mangle]
pub fn test_to_hex(x: u64) -> SmolStr {
    HexidecimalInt(x).to_hex()
}

#[inline(never)]
#[no_mangle]
pub fn test_to_string(x: u64) -> String {
    HexidecimalInt(x).to_string()
}

fn main() {}
