use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use regex_automata::{DenseDFA, Regex, RegexBuilder};

lazy_static::lazy_static! {
    static ref PROTOCOLS: AhoCorasick = AhoCorasickBuilder::new().dfa(true).match_kind(MatchKind::LeftmostFirst).build(&[
        "https://", "http://"
    ]);

    static ref URL: Regex<DenseDFA<Vec<u16>, u16>> = RegexBuilder::new()
        .minimize(true)
        .anchored(true) // Using AhoCorasick, we're already at the start of the substr
        .build_with_size::<u16>(r#"[^\s<]+[^<.,:;"')\]\s]"#)
        .unwrap();

    static ref URL2: Regex<DenseDFA<Vec<u16>, u16>> = RegexBuilder::new()
        .minimize(true)
        .build_with_size::<u16>(r#"https?://[^\s<]+[^<.,:;"')\]\s]"#)
        .unwrap();

    static ref URL3: Regex<DenseDFA<Vec<u16>, u16>> = RegexBuilder::new()
        .minimize(true)
        .anchored(true)
        .build_with_size::<u16>(r#"https?://[^\s<]+[^<.,:;"')\]\s]"#)
        .unwrap();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Url<'a> {
    pub secure: bool,
    pub url: &'a str,
}

pub type UrlList<'a> = smallvec::SmallVec<[Url<'a>; 4]>;

pub fn find_urls_aho_corasick<'a>(input: &'a str) -> UrlList<'a> {
    let bytes = input.as_bytes();

    let mut res = UrlList::default();
    let mut state = FreeState::new();

    for m in PROTOCOLS.find_iter(bytes) {
        if !state.is_free(input, m.start()) {
            continue;
        }

        if let Some((_url_start, mut url_end)) = URL.find(&bytes[m.end()..]) {
            // Note that the URL ends relative to m.end()
            url_end += m.end();

            let url_sub = unsafe { bytes.get_unchecked(m.start()..url_end) };

            res.push(Url {
                secure: m.pattern() == 0,
                url: match std::str::from_utf8(url_sub) {
                    Ok(url) => {
                        // fast-forward through URL
                        state.position = url_end;

                        url
                    }
                    Err(_) => continue, // ignore bad URL
                },
            })
        }
    }

    res
}

use memchr::memmem::find_iter;

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

        if let Some((_, mut url_end)) = URL.find(&bytes[end..]) {
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

pub fn find_urls_multiple_memchr<'a>(input: &'a str) -> UrlList<'a> {
    let bytes = input.as_bytes();

    let mut res = UrlList::default();
    let mut state = FreeState::new();

    let mut http = find_iter(bytes, "http://");
    let mut https = find_iter(bytes, "https://");

    loop {
        let mut nhttp = http.next();
        let mut nhttps = https.next();

        if (nhttp, nhttps) == (None, None) {
            break;
        }

        loop {
            // take the current lowest position and iterate
            let start = match (nhttp, nhttps) {
                (Some(nhttp_pos), Some(nhttps_pos)) => {
                    if nhttp_pos < nhttps_pos {
                        nhttp = http.next();
                        nhttp_pos
                    } else {
                        nhttps = https.next();
                        nhttps_pos
                    }
                }
                (Some(nhttp_pos), None) => {
                    nhttp = http.next();
                    nhttp_pos
                }
                (None, Some(nhttps_pos)) => {
                    nhttps = https.next();
                    nhttps_pos
                }
                _ => break,
            };

            if !state.is_free(input, start) {
                continue;
            }

            if let Some((_, mut end)) = URL3.find(&bytes[start..]) {
                end += start;

                state.position = end;

                let substr = &input[start..end];

                res.push(Url {
                    secure: substr.starts_with("https"),
                    url: substr,
                });
            }
        }
    }

    res
}

pub fn find_urls_regex_only<'a>(input: &'a str) -> UrlList<'a> {
    let mut res = UrlList::default();

    let mut state = FreeState::new();

    for (start, end) in URL2.find_iter(input.as_bytes()) {
        if !state.is_free(input, start) {
            continue;
        }

        // fast-forward
        state.position = end;

        let substr = &input[start..end];

        res.push(Url {
            secure: substr.starts_with("https"),
            url: substr,
        });
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
            if c == b'`' {
                self.consecutive_code += 1;
            } else {
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

        // escaped URL: <https://test.com>
        if matches!(input.as_bytes()[new_position - 1], b'<' | b'\\') {
            return false;
        }

        self.increment(input, new_position);

        if !self.flags.is_empty() {
            return false;
        }

        true
    }
}
