use once_cell::sync::Lazy;
use regex::{Match, Regex, RegexBuilder};

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
    EMOJI_INDICES
        .windows(2)
        .map(|i| &EMOJIS[(i[0] as usize)..(i[1] as usize)])
}

/// NOTE: This will match `^[*#0-9]$` as well, so double-check results
///
/// <https://www.unicode.org/reports/tr51/tr51-22.html#EBNF_and_Regex>
pub static EMOJI_RE: Lazy<Regex> = Lazy::new(|| {
    RegexBuilder::new(
        r#"
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
    "#,
    )
    .ignore_whitespace(true)
    .unicode(true)
    .build()
    .unwrap()
});

/// Finds emojis in the string *and* filters out single strings of `/#*[0-9]/`
pub fn find_emojis(e: &str) -> impl Iterator<Item = Match> {
    EMOJI_RE.find_iter(e).filter(|m| {
        !((m.end() - m.start()) == 1 && matches!(m.as_str().as_bytes()[0], b'#' | b'*' | b'0'..=b'9'))
    })
}
