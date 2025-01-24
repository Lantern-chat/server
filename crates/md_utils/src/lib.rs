use std::borrow::Cow;
use std::ops::{Range, RangeInclusive};

use smallvec::SmallVec;

pub mod regexes {
    // TODO: Replace with custom iterator?
    regex_automata_macro::decl_regex_sparse! {
        pub NEWLINES = r"(\r?\n){3,}"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpanType {
    None,
    InlineCode,
    BlockCode,
    InlineMath,
    BlockMath,
    Url,
    CustomEmote,
    UserMention,
    RoomMention,
    Spoiler,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    start: usize,
    len: u16,
    kind: SpanType,
}

impl Span {
    pub const fn start(&self) -> usize {
        self.start
    }

    pub const fn end(&self) -> usize {
        self.start() + self.len()
    }

    pub const fn len(&self) -> usize {
        self.len as usize
    }

    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub const fn range(&self) -> Range<usize> {
        self.start..self.end()
    }

    pub const fn kind(&self) -> SpanType {
        self.kind
    }
}

pub type SpanList = SmallVec<[Span; 32]>;

pub fn is_spoilered(spans: &[Span], idx: usize) -> bool {
    for span in spans {
        if span.kind == SpanType::Spoiler && span.range().contains(&idx) {
            return true;
        }
    }
    false
}

#[inline]
fn valid_url(c: &char) -> bool {
    c.is_ascii()
        && matches!(*c as u8, b'A'..=b'Z' | b'a'..=b'z' | b'#'..=b';' /*includes digits*/ | b'!' | b'=' | b'?' | b'@' | b'[' | b']' | b'_' | b'~')
}

pub fn scan_markdown(input: &str) -> SpanList {
    let mut spans = SpanList::default();

    scan_markdown_recursive::<true>(input, 0, &mut spans);

    spans
}

#[inline(always)]
fn scan_markdown_recursive<const S: bool>(input: &str, offset: usize, spans: &mut SpanList) {
    let bytes = input.as_bytes();
    let mut chars = input.char_indices();
    let mut escaped = false;

    macro_rules! new_span {
        ($prefix_len: expr, $start:expr, $len:expr, $kind:expr) => {{
            let start = $start + offset + $prefix_len;

            spans.push(Span {
                start,
                len: $len as u16,
                kind: $kind,
            });
        }};
    }

    let mut last_char = 0 as char;

    while let Some((i, c)) = chars.next() {
        if escaped || c == '\\' {
            escaped ^= true;
            continue;
        }

        let lc = last_char;
        last_char = c;

        if !c.is_ascii() {
            continue;
        }

        // SAFETY: This is a known valid index (given by `.char_indices()`)
        // inside `input` and therefore into `bytes`
        let slice = unsafe { bytes.get_unchecked(i..) };

        let skip = match slice {
            // start of code block, and not two zero-length inline codes
            [b'`', b'`', b'`', rest @ ..] if !rest.starts_with(b"`") => scan_substr(
                3,
                rest,
                Some("\n```"), // NOTE: This includes a newline, so there must be at least one newline
                |_| true,
                |len, _| new_span!(3, i, len, SpanType::BlockCode),
            ),

            // empty inline code span
            [b'`', b'`', ..] => 2,

            // inline code
            [b'`', rest @ ..] => scan_substr(
                1,
                rest,
                Some("`"),
                |c| *c != '\n',
                |len, _| new_span!(1, i, len, SpanType::InlineCode),
            ),

            // mention
            [b'<', rest @ ..] => match rest {
                [n @ (b'@' | b'#'), rest @ ..] => {
                    scan_substr(2, rest, Some(">"), char::is_ascii_digit, |len, _| {
                        let kind = match n {
                            b'@' => SpanType::UserMention,
                            b'#' => SpanType::RoomMention,
                            _ => unsafe { std::hint::unreachable_unchecked() },
                        };
                        new_span!(2, i, len, kind);
                    })
                }
                [b':', rest @ ..] => {
                    use std::cell::Cell;

                    let hit_sep = Cell::new(false);
                    let has_id = Cell::new(false);

                    let valid = |c: &char| -> bool {
                        match c {
                            'a'..='z' | 'A'..='Z' if !hit_sep.get() => true,
                            ':' => !hit_sep.replace(true), // if we hit : already, then this is invalid
                            '0'..='9' => {
                                has_id.set(hit_sep.get());

                                true
                            }
                            _ => false,
                        }
                    };

                    let skip = scan_substr(2, rest, Some(">"), valid, |len, _| {
                        if hit_sep.get() && has_id.get() {
                            new_span!(2, i, len, SpanType::CustomEmote);
                        }
                    });

                    match hit_sep.get() && has_id.get() {
                        true => skip,
                        false => 0,
                    }
                }
                [b'h', b't', b't', b'p', rest @ ..]
                    if matches!(rest, [b's', b':', b'/', b'/', ..] | [b':', b'/', b'/', ..]) =>
                {
                    scan_substr(1 + 4 + 3, &rest[3..], Some(">"), valid_url, |_, _| {})
                }
                _ => 0,
                // TODO: Investigate this again?
                // skip anything within <...> if it doesn't have whitespace
                //_ => scan_substr(1, rest, Some(">"), |c| !c.is_whitespace(), |_, _| {}),
            },

            // link
            [b'h', b't', b't', b'p', rest @ ..]
                if !lc.is_alphanumeric() // enforce word-boundary rules
                    && matches!(rest, [b's', b':', b'/', b'/', ..] | [b':', b'/', b'/', ..]) =>
            {
                scan_substr(4 + 3, &rest[3..], None, valid_url, |len, _| {
                    new_span!(0, i, len + 4 + 3, SpanType::Url)
                })
            }

            // start of spoiler span
            [b'|', b'|', rest @ ..] if S => scan_substr(
                2,
                rest,
                Some("||"),
                |_| true,
                |len, span| {
                    new_span!(2, i, len, SpanType::Spoiler);
                    scan_markdown_recursive::<false>(span, i + 2 + offset, spans);
                },
            ),

            // // block math
            // [b'$', b'$', rest @ ..] => todo!("math block"),

            // // likely inline math or single dollar sign
            // [b'$', ..] => todo!("math"),
            _ => 0,
        };

        if skip > 1 {
            chars.nth(skip - 1); // advance_by
        }
    }
}

fn scan_substr(
    prefix_length: usize,
    input: &[u8],
    until: Option<&str>,
    valid: impl Fn(&char) -> bool,
    on_found: impl FnOnce(usize, &str),
) -> usize {
    let mut len = 0;
    scan_substr_inner(input, until, valid, |end| {
        len = end;
        on_found(len, unsafe { std::str::from_utf8_unchecked(&input[0..end]) });
    });

    match until {
        Some(u) => prefix_length + len + u.len() - 1,
        None => 1,
    }
}

fn scan_substr_inner(
    input: &[u8],
    until: Option<&str>,
    valid: impl Fn(&char) -> bool,
    on_found: impl FnOnce(usize),
) {
    #[cfg(debug_assertions)]
    let Ok(input) = std::str::from_utf8(input) else {
        eprintln!("Input is not valid UTF8!");
        return;
    };

    // SAFETY: This *should* be safe, given that the slice given in
    // calling functions is constructed from char_indices or ASCII slice matching
    #[cfg(not(debug_assertions))]
    let input = unsafe { std::str::from_utf8_unchecked(input) };

    let has_until = until.is_some();
    let until_bytes = until.map(|u| u.as_bytes());

    let bytes = input.as_bytes();
    let mut escaped = false;

    for (i, c) in input.char_indices() {
        if escaped || c == '\\' {
            escaped ^= true;
            continue;
        }

        if let Some(until) = until_bytes {
            // SAFETY: This is a known valid index (given by `.char_indices()`)
            // inside `input` and therefore into `bytes`
            let slice = unsafe { bytes.get_unchecked(i..) };
            if slice.starts_with(until) {
                return on_found(i);
            }
        }

        if !valid(&c) {
            if !has_until {
                on_found(i);
            }

            return;
        }
    }

    if !has_until {
        on_found(input.len());
    }
}

#[derive(Debug, Clone)]
pub struct TrimLimits {
    pub len: RangeInclusive<usize>,
    pub max_newlines: usize,
}

/// If the message has more than 2 consecutive newlines, this will strip those and replace them with just two.
///
/// Carriage returns will be ignored unless it follows under the above condition, in which case they will be removed.
pub fn trim_message(content: &str, limits: Option<TrimLimits>) -> Option<Cow<str>> {
    let mut trimmed_content = Cow::Borrowed(content.trim());

    if !trimmed_content.is_empty() {
        let mut new_content = String::new();
        let mut idx = 0;

        for m in regexes::NEWLINES.find_iter(trimmed_content.as_bytes()) {
            let r = m.range();
            new_content.push_str(&trimmed_content[idx..r.start]);
            new_content.push_str("\n\n");
            idx = r.end;
        }

        if idx != 0 {
            new_content.push_str(&trimmed_content[idx..]);

            // trim any ending whitespace
            new_content.truncate(new_content.trim_end().len());

            trimmed_content = new_content.into();
        }

        if let Some(limits) = limits {
            let newlines = bytecount::count(trimmed_content.as_bytes(), b'\n');

            let too_large = !limits.len.contains(&trimmed_content.len());
            let too_long = newlines > limits.max_newlines;

            if too_large || too_long {
                return None;
            }
        }
    }

    Some(trimmed_content)
}

#[cfg(test)]
mod url_test {
    use super::*;

    const VALID_URL: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-%._~:/?#[]@!$&'()*+,;=";

    #[test]
    fn test_valid_url() {
        for c in 0..256 {
            let c = char::from_u32(c).unwrap();

            assert_eq!(VALID_URL.contains(&(c as u8)), valid_url(&c), "Invalid? {}", c as u8);
        }
    }

    #[test]
    fn test_newlines_size() {
        println!("{} + {}", regexes::NEWLINES.forward().memory_usage(), regexes::NEWLINES.reverse().memory_usage());
    }
}
