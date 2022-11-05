use memchr::memmem::find_iter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Url<'a> {
    pub secure: bool,
    pub url: &'a str,
}

pub type UrlList<'a> = smallvec::SmallVec<[Url<'a>; 4]>;

pub fn find_urls<'a>(input: &'a str) -> UrlList<'a> {
    let bytes = input.as_bytes();

    let mut res = UrlList::default();
    let mut state = FreeState::new();

    for start in find_iter(bytes, "http") {
        let mut end = start + 4; // "http"

        let secure = match &input[end..(end + 3)] {
            // http://
            "://" => {
                end += 3;
                false
            }

            // https://
            "s:/" if bytes[end + 3] == b'/' => {
                end += 4;
                true
            }
            _ => continue,
        };

        if !state.is_free(input, start) {
            continue;
        }

        if let Some((_, mut url_end)) = crate::regexes::URL.find(&bytes[end..]) {
            url_end += end;

            state.position = url_end;

            res.push(Url {
                secure,
                url: &input[start..url_end],
            });
        }
    }

    res
}

pub fn is_free(input: &str, pos: usize) -> bool {
    FreeState::new().is_free(input, pos)
}

bitflags::bitflags! {
    struct Flags: u8 {
        const INSIDE_CODE_BLOCK     = 1 << 0;
        const INSIDE_SPOILER        = 1 << 1;
        const INSIDE_INLINE_CODE    = 1 << 2;
        const ESCAPED               = 1 << 4;
    }
}

struct FreeState {
    consecutive_spoiler: u32,
    consecutive_code: u32,
    flags: Flags,
    position: usize,
}

impl FreeState {
    const fn new() -> Self {
        FreeState {
            consecutive_spoiler: 0,
            consecutive_code: 0,
            flags: Flags::empty(),
            position: 0,
        }
    }

    fn increment(&mut self, input: &str, new_position: usize) {
        debug_assert!(self.position <= new_position);

        // trim to avoid over-processing
        let input = unsafe { input.get_unchecked(self.position..new_position) };

        for c in input.bytes() {
            if self.flags.contains(Flags::ESCAPED) {
                self.flags.remove(Flags::ESCAPED);
                continue;
            }

            if c == b'\\' {
                self.flags.insert(Flags::ESCAPED);
                continue;
            }

            if c == b'`' {
                self.consecutive_code += 1;
            } else {
                // if this character is not part of a code token,
                // but there were two consecitive code tokens,
                // then it was probably a zero-length inline code span
                if self.consecutive_code == 2 && !self.flags.contains(Flags::INSIDE_CODE_BLOCK) {
                    self.flags.toggle(Flags::INSIDE_INLINE_CODE);
                }

                self.consecutive_code = 0;
            }

            if c == b'|' {
                self.consecutive_spoiler += 1;
            } else {
                self.consecutive_spoiler = 0;
            }

            if self.consecutive_code == 3 {
                self.flags.toggle(Flags::INSIDE_CODE_BLOCK);

                // either entering or exiting a code block, either way not inline
                self.flags.remove(Flags::INSIDE_INLINE_CODE);
            } else if !self.flags.contains(Flags::INSIDE_CODE_BLOCK) {
                if self.consecutive_code == 1 {
                    self.flags.toggle(Flags::INSIDE_INLINE_CODE);
                }

                if self.consecutive_spoiler == 2 {
                    self.flags.toggle(Flags::INSIDE_SPOILER);
                }
            }
        }

        self.position = new_position;
    }

    fn is_free(&mut self, input: &str, new_position: usize) -> bool {
        // start of message is URL
        if new_position == 0 {
            return true;
        }

        // escaped URL: <https://test.com> or prefixed to escape or be part of emote
        if matches!(input.as_bytes()[new_position - 1], b'<' | b'\\' | b':') {
            return false;
        }

        self.increment(input, new_position);

        if !self.flags.is_empty() {
            return false;
        }

        true
    }
}
