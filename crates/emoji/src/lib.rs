include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

/// Takes any valid form (and some invalid) of an emoji and returns the fully-qualified form
pub fn find(e: &str) -> Option<&'static str> {
    match FORMS_TO_INDEX.get(e) {
        Some(&idx) => {
            let idx = idx as usize;
            let start = EMOJI_INDICES[idx] as usize;
            let end = EMOJI_INDICES[idx + 1] as usize;

            EMOJIS.get(start..end)
        }
        None => None,
    }
}

/// Iterates through all fully-qualified emojis
pub fn iter() -> impl Iterator<Item = &'static str> {
    EMOJI_INDICES.windows(2).map(|i| unsafe { EMOJIS.get_unchecked((i[0] as usize)..(i[1] as usize)) })
}

use once_cell::sync::Lazy;
use regex_automata::{Regex, RegexBuilder};

/// NOTE: This will match `^[*#0-9]$` as well, so double-check results
///
/// <https://www.unicode.org/reports/tr51/tr51-22.html#EBNF_and_Regex>

// DEV NOTE: The validity of u16 here is subject to change as the number of emojis increases
pub static EMOJI_RE: Lazy<Regex<regex_automata::DenseDFA<Vec<u16>, u16>>> = Lazy::new(|| {
    RegexBuilder::new()
        .minimize(true)
        .ignore_whitespace(true)
        .unicode(true)
        .build_with_size(
            r"
        \p{RI} \p{RI}
        | \p{Emoji}
            ( \p{EMod}
            | \x{FE0F} \x{20E3}?
            | [\x{E0020}-\x{E007E}]+ \x{E007F}
            )?
            (\x{200D}
                ( \p{RI} \p{RI}
                    | \p{Emoji}
                    ( \p{EMod}
                    | \x{FE0F} \x{20E3}?
                    | [\x{E0020}-\x{E007E}]+ \x{E007F}
                    )?
                )
            )*
        ",
        )
        .unwrap()
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match<'a> {
    input: &'a str,
    start: usize,
    end: usize,
}

impl<'a> Match<'a> {
    #[inline]
    pub fn as_str(&self) -> &'a str {
        // SAFETY: This range was created from regex-automata's `find_iter`,
        // which ensures it was within the input string
        unsafe { self.input.get_unchecked(self.start..self.end) }
    }

    #[inline]
    pub const fn start(&self) -> usize {
        self.start
    }

    #[inline]
    pub const fn end(&self) -> usize {
        self.end
    }
}

/// Finds emojis in the string *and* filters out single strings of `/#*[0-9]/`
pub fn find_emojis(input: &str) -> impl Iterator<Item = Match> {
    EMOJI_RE.find_iter(input.as_bytes()).filter_map(|(start, end)| {
        if (end - start) == 1
            && matches!(
                // SAFETY: regex-automata `find_iter` ensures `start` is within the input string
                unsafe { input.as_bytes().get_unchecked(start) },
                b'#' | b'*' | b'0'..=b'9'
            )
        {
            return None;
        }

        Some(Match { input, start, end })
    })
}
