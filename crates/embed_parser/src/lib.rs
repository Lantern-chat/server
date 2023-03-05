#[macro_use]
extern crate serde;

pub mod embed;
pub mod html;
pub mod oembed;
pub mod quirks;
pub mod utils;

#[inline]
fn trim_quotes(s: &str) -> &str {
    s.trim_matches(|c: char| ['"', '\'', '“', '”'].contains(&c) || c.is_whitespace())
}

pub mod regexes {
    use regex_automata::{DenseDFA, Regex};

    include!(concat!(env!("OUT_DIR"), "/codegen.rs"));
}

/// We can't embed infinite text, so this attempts to trim it below `max_len` without abrubtly
/// cutting off. It will find punctuation nearest to the limit and trim to there, or
pub fn trim_text(mut text: &str, max_len: usize) -> &str {
    text = text.trim(); // basic ws trim first

    if text.len() <= max_len {
        return text;
    }

    text = &text[..max_len];

    // try to find punctuation
    for (idx, char) in text.char_indices().rev() {
        if matches!(char, '.' | ',' | '!' | '?' | '\n') {
            return text[..idx].trim_end();
        }
    }

    text
}
