use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use regex_automata::{DenseDFA, Regex, RegexBuilder};

lazy_static::lazy_static! {
    static ref HTTP: AhoCorasick = AhoCorasickBuilder::new().dfa(true).match_kind(MatchKind::LeftmostFirst).build(&[
        "http://", "https://"
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
}

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

    for m in HTTP.find_iter(bytes) {
        if !state.is_free(&input, m.start()) {
            continue;
        }

        if let Some((_url_start, mut url_end)) = URL.find(&bytes[m.end()..]) {
            // Note that the URL ends relative to m.end()
            url_end += m.end();

            let url_sub = unsafe { bytes.get_unchecked(m.start()..url_end) };

            res.push(Url {
                secure: m.pattern() == 1,
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

pub fn find_urls2<'a>(input: &'a str) -> UrlList<'a> {
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

struct FreeState {
    consecutive_spoiler: u32,
    consecutive_code: u32,
    inside_code_block: bool,
    inside_spoiler: bool,
    inside_inline_code: bool,
    position: usize,
}

impl FreeState {
    const fn new() -> Self {
        FreeState {
            consecutive_spoiler: 0,
            consecutive_code: 0,
            inside_code_block: false,
            inside_spoiler: false,
            inside_inline_code: false,
            position: 0,
        }
    }

    fn increment(&mut self, input: &str, new_position: usize) {
        // trim to avoid over-processing
        let input = &input[self.position..new_position];

        for c in input.chars() {
            if c == '`' {
                self.consecutive_code += 1;
            } else {
                self.consecutive_code = 0;
            }

            if c == '|' {
                self.consecutive_spoiler += 1;
            } else {
                self.consecutive_spoiler = 0;
            }

            if self.consecutive_code == 3 {
                self.inside_code_block ^= true;

                if self.inside_code_block {
                    self.inside_inline_code = false;
                }
            }

            if !self.inside_code_block {
                if self.consecutive_code == 1 {
                    self.inside_inline_code ^= true;
                }

                if self.consecutive_spoiler == 2 {
                    self.inside_spoiler ^= true;
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

        if self.inside_code_block || self.inside_spoiler || self.inside_inline_code {
            return false;
        }

        true
    }
}
