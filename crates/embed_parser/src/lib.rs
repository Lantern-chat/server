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
