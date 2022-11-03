use embed_parser::msg::{is_free, Url};
use smallvec::SmallVec;

#[inline(never)]
#[no_mangle]
pub fn asm_is_free(input: &str, pos: usize) -> bool {
    assert!(pos < input.len());

    is_free(input, pos)
}

#[inline(never)]
#[no_mangle]
pub fn find_urls(input: &str) -> SmallVec<[Url; 4]> {
    embed_parser::msg::find_urls(input)
}

fn main() {}
