use embed_parser::msg::is_free;

#[inline(never)]
#[no_mangle]
pub fn asm_is_free(input: &str, pos: usize) -> bool {
    assert!(pos < input.len());

    is_free(input, pos)
}

fn main() {}
