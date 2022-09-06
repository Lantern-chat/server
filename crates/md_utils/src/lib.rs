use smallvec::SmallVec;

/// By definition, these are all non-overlapping spans
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
    pub start: usize,
    pub end: usize,
    pub kind: SpanType,
}

pub type SpanList = SmallVec<[Span; 16]>;

pub fn is_spoilered(spans: &[Span], idx: usize) -> bool {
    for span in spans {
        if span.kind == SpanType::Spoiler && span.start <= idx && idx < span.end {
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

#[cfg(test)]
mod url_test {
    use super::*;

    const VALID_URL: &'static [u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-%._~:/?#[]@!$&'()*+,;=";

    #[test]
    fn test_valid_url() {
        for c in 0..256 {
            let c = char::from_u32(c).unwrap();

            assert_eq!(
                VALID_URL.contains(&(c as u8)),
                valid_url(&c),
                "Invalid? {}",
                c as u8
            );
        }
    }
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
            let end = start + $len;

            //println!("Span {}..{} of {:?}", start, end, $kind);
            spans.push(Span {
                start,
                end,
                kind: $kind,
            });
        }};
    }
    while let Some((i, c)) = chars.next() {
        if escaped || c == '\\' {
            escaped ^= true;
            continue;
        }

        if !c.is_ascii() {
            continue;
        }

        let slice = &bytes[i..];

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
            [b'<', n, rest @ ..] => {
                if b":@#".contains(n) {
                    scan_substr(2, rest, Some(">"), char::is_ascii_digit, |len, _| {
                        new_span!(
                            2,
                            i,
                            len,
                            match n {
                                b':' => SpanType::CustomEmote,
                                b'@' => SpanType::UserMention,
                                b'#' => SpanType::RoomMention,
                                _ => unreachable!(),
                            }
                        )
                    })
                } else if *n == b'h' && rest.starts_with(b"ttp") {
                    scan_substr(1, &slice[1..], Some(">"), valid_url, |_, _| {})
                } else {
                    0
                }
            }

            // link
            [b'h', b't', b't', b'p', s, ..] if b"s:".contains(s) => {
                scan_substr(0, slice, None, valid_url, |len, _| {
                    new_span!(0, i, len, SpanType::Url)
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
    let input = match std::str::from_utf8(input) {
        Ok(input) => input,
        Err(_) => {
            eprintln!("Input is not valid UTF8!");
            return;
        }
    };

    #[cfg(not(debug_assertions))]
    let input = unsafe { std::str::from_utf8_unchecked(input) };

    let bytes = input.as_bytes();
    let mut escaped = false;

    let mut chars = input.char_indices();

    while let Some((i, c)) = chars.next() {
        if escaped || c == '\\' {
            escaped ^= true;
            continue;
        }

        if let Some(until) = until {
            if bytes[i..].starts_with(until.as_bytes()) {
                return on_found(i);
            }
        }

        if !valid(&c) {
            if until.is_some() {
                break;
            } else {
                return on_found(i);
            }
        }
    }

    if until.is_none() {
        on_found(input.len());
    }
}
