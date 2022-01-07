bitflags::bitflags! {
    struct Flags: u8 {
        const ESCAPED = 1 << 0;
        const STRING = 1 << 1;
    }
}

pub struct SplitWhitespaceEscaped<'a> {
    str: &'a [u8],
    idx: usize,
    flags: Flags,
}

impl<'a> SplitWhitespaceEscaped<'a> {
    pub fn new(str: &'a str) -> Self {
        SplitWhitespaceEscaped {
            str: str.as_bytes(),
            idx: 0,
            flags: Flags::empty(),
        }
    }

    fn incr(&mut self) {
        let mut offset = 0;

        for &c in &self.str[self.idx..] {
            offset += 1;

            // if in an escape, remove it and skip the current character
            if self.flags.contains(Flags::ESCAPED) {
                self.flags.remove(Flags::ESCAPED);
                continue;
            }

            if c == b'"' {
                // entering or exiting a string
                self.flags.toggle(Flags::STRING);
            } else if self.flags.contains(Flags::STRING) {
                // inside a string, characters can be escaped
                if c == b'\\' {
                    self.flags.insert(Flags::ESCAPED);
                }
            } else if c.is_ascii_whitespace() {
                // outside of a string, whitespace is a breakpoint
                break;
            }
        }

        self.idx += offset;
    }
}

//impl<'a> Iterator for SplitWhitespaceEscaped<'a> {
//    type Item = &'a str;
//}
