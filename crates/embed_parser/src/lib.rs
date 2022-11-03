#[macro_use]
extern crate serde;

pub mod embed;
pub mod html;
//pub mod iter;
pub mod msg;
pub mod oembed;
//pub mod req;

#[inline]
fn trim_quotes(s: &str) -> &str {
    s.trim_matches(|c| c == '"' || c == '\'')
}

pub mod regexes {
    use regex_automata::{DenseDFA, Regex};

    include!(concat!(env!("OUT_DIR"), "/codegen.rs"));
}
