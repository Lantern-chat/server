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

use std::sync::LazyLock;

/// NOTE: This will match `^[*#0-9]$` as well, so double-check results
///
/// <https://www.unicode.org/reports/tr51/#EBNF_and_Regex>
pub static EMOJI_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"(?ux)
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

/// Finds emojis in the string *and* filters out single strings of `/#*[0-9]/`
pub fn find_emojis(input: &str) -> impl Iterator<Item = regex::Match<'_>> {
    EMOJI_RE.find_iter(input).filter(|m| {
        if m.len() == 1 && matches!(m.as_str().as_bytes()[0], b'#' | b'*' | b'0'..=b'9') {
            return false;
        }

        true
    })
}
