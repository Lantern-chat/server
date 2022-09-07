use std::borrow::Cow;

use aho_corasick::AhoCorasick;

use crate::Error;

lazy_static::lazy_static! {
    static ref SLASH_PATTERNS: AhoCorasick = aho_corasick::AhoCorasickBuilder::new()
    .dfa(true).anchored(true).match_kind(aho_corasick::MatchKind::LeftmostFirst)
    // include a space at the end of the name
    .build(Pattern::NAMES);
}

#[allow(unused)]
macro_rules! decl_patterns {
    ($first:ident $(, $rest:ident)*) => {paste::paste! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[repr(u8)]
        pub enum Pattern {
            $first = 0,
            $($rest,)*
        }

        impl Pattern {
            const NAMES: &'static [&'static str] = &[ stringify!([<$first:lower>]) $(, stringify!([<$rest:lower>]) )* ];

            fn from_index(idx: usize) -> Pattern {
                const ALL: &'static [Pattern] = &[Pattern::$first $(, Pattern::$rest)*];

                return ALL[idx];
            }
        }
    }}
}

decl_patterns! {
    Gimme, Shrug, TableFlip, Unflip, Lenny, Disapprove, Me, Spoiler
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Align {
    Left,
    Right,
}

pub fn process_slash(content: &str, _active: bool) -> Result<Option<Cow<str>>, Error> {
    let unchanged = Ok(Some(Cow::Borrowed(content)));

    if !content.starts_with('/') {
        return unchanged;
    }

    let bytes = &content.as_bytes()[1..];

    if let Some(m) = SLASH_PATTERNS.earliest_find(&bytes) {
        let mut end_idx = m.end();
        let mut do_command = false;

        if end_idx == bytes.len() {
            do_command = true;
        } else if bytes[end_idx].is_ascii_whitespace() {
            end_idx += 1; // also consume the whitespace
            do_command = true;
        }

        if !do_command {
            return unchanged;
        }

        // skip past the leading slash + match + any extra whitespace
        let content = content[1 + end_idx..].trim_start();

        let (align, value) = match Pattern::from_index(m.pattern()) {
            Pattern::Gimme => (Align::Left, "༼ つ ◕_◕ ༽つ"),
            Pattern::Lenny => (Align::Right, "( ͡° ͜ʖ ͡°)"),
            Pattern::Shrug => (Align::Right, "¯\\\\_(ツ)_/¯"),
            Pattern::TableFlip => (Align::Right, "(╯°□°）╯︵ ┻━┻"),
            Pattern::Unflip => (Align::Right, "┬─┬ ノ( ゜-゜ノ)"),
            Pattern::Disapprove => (Align::Right, "ಠ_ಠ"),
            Pattern::Me => return Ok(Some(format!("_{content}_").into())),
            Pattern::Spoiler => return Ok(Some(format!("||{content}||").into())),
            // TODO: If it's a slash command that actually does something, only do it when `active` is true
        };

        // nothing to pad
        if content.is_empty() {
            return Ok(Some(value.into()));
        }

        let mut left = content;
        let mut right = value;

        // if value is supposed to be on the left, swap
        if align == Align::Left {
            std::mem::swap(&mut left, &mut right);
        }

        return Ok(Some(format!("{left} {right}").into()));
    }

    return unchanged;
}
