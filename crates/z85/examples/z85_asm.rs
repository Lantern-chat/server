fn main() {}

use z85::{ParseZ85, ToZ85};

#[no_mangle]
#[inline(never)]
pub fn test_to_z85(data: &[u8]) -> String {
    data.to_z85().unwrap()
}
