use std::borrow::Cow;
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use indexmap::map::{Entry, IndexMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Qualified {
    Fully,
    Minimal,
    None,
    Component,
}

impl Qualified {
    pub const fn as_str(self) -> &'static str {
        match self {
            Qualified::Fully => "fully-qualified",
            Qualified::Minimal => "minimally-qualified",
            Qualified::None => "unqualified",
            Qualified::Component => "component",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedEmoji<'a> {
    pub emoji: &'a str,
    pub name: &'a str,
    pub status: Qualified,
    pub forms: Vec<&'a str>,
}

/// Removes any `\u{FE0F}` variation selectors for a more compact representation
pub fn minimize<'a>(s: &'a str) -> Cow<'a, str> {
    if !s.contains('\u{200D}') {
        s.replace("\u{FE0F}", "").into()
    } else {
        s.into()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(&env::var("OUT_DIR")?).join("codegen.rs");
    let mut file = BufWriter::new(File::create(&path)?);

    let src = include_str!("./emoji-test.txt");
    let mut emojis = IndexMap::new();

    for line in src.lines().chain(EXTRA.lines()) {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        // codepoints don't matter, we can just use the emoji symbol itself
        let line = &line.trim_start_matches(|c| c != ';')[2..];

        let status = if line.starts_with(Qualified::Fully.as_str()) {
            Qualified::Fully
        } else if line.starts_with(Qualified::None.as_str()) {
            Qualified::None
        } else if line.starts_with(Qualified::Minimal.as_str()) {
            Qualified::Minimal
        } else if line.starts_with(Qualified::Component.as_str()) {
            Qualified::Component
        } else {
            panic!("Unknown qualification: {}", line);
        };

        // trim off any whitespace and # before the emoji itself
        let line =
            line[status.as_str().len()..].trim_start_matches(|c: char| c.is_ascii_whitespace() || c == '#');

        // find the end of the emoji
        let emoji_end_idx = line.find(" E").expect("End of emoji");

        let emoji = &line[..emoji_end_idx];

        // go past the emoji + ' E', trim unicode version (it's not whitespace), then trim whitespace to get the full name
        let name = line[(emoji_end_idx + 2)..]
            .trim_start_matches(|c: char| !c.is_ascii_whitespace())
            .trim();

        match emojis.entry(name) {
            Entry::Vacant(v) => {
                v.insert(ParsedEmoji {
                    emoji,
                    name,
                    status,
                    forms: Vec::new(),
                });
            }
            Entry::Occupied(mut o) => {
                if status == Qualified::Fully {
                    let e = o.get().emoji;

                    let old = o.insert(ParsedEmoji {
                        emoji,
                        name,
                        status,
                        forms: vec![e],
                    });

                    o.get_mut().forms.extend(old.forms);
                } else {
                    o.get_mut().forms.push(emoji);
                }
            }
        }
    }

    let mut forms_to_full: IndexMap<Cow<'_, str>, &str> = IndexMap::new();
    let mut emoji_to_idx: IndexMap<&str, usize> = IndexMap::new();
    let mut indices: Vec<usize> = Vec::new();
    let mut merged = String::new();

    let mut add_emoji = |e: &str| {
        let start = merged.len();
        merged.push_str(e);

        let idx = indices.len();
        indices.push(start);

        idx
    };

    for e in emojis.values() {
        emoji_to_idx.insert(e.emoji, add_emoji(e.emoji));

        forms_to_full.insert(e.emoji.into(), e.emoji);
        forms_to_full.insert(minimize(e.emoji), e.emoji);

        for &form in &e.forms {
            forms_to_full.insert(form.into(), e.emoji);
            forms_to_full.insert(minimize(form), e.emoji);
        }
    }

    // so we can use `.windows(2)` in the library code
    indices.push(merged.len());

    let idx_ty = if merged.len() < u16::MAX as usize { "u16" } else { "u32" };

    write!(file, "static EMOJIS: &'static str = \"{}\";\n", merged)?;
    write!(
        file,
        "static EMOJI_INDICES: [{idx_ty}; {}] = {:?};\n",
        indices.len(),
        indices
    )?;

    let mut forms_to_full_map = phf_codegen::Map::new();

    for (form, emoji) in forms_to_full.iter() {
        let idx = emoji_to_idx
            .get(emoji)
            .unwrap_or_else(|| panic!("Indices not found for emoji: {}", emoji));

        forms_to_full_map.entry(form.as_ref(), &idx.to_string());
    }

    write!(
        file,
        "static FORMS_TO_INDEX: phf::Map<&'static str, u16> = {};\n",
        forms_to_full_map.build()
    )?;

    Ok(())
}

static EXTRA: &'static str = "
1F1E6 ; fully-qualified # \u{1F1E6} E6.0 Regional Indicator Symbol Letter A
1F1E7 ; fully-qualified # \u{1F1E7} E6.0 Regional Indicator Symbol Letter B
1F1E8 ; fully-qualified # \u{1F1E8} E6.0 Regional Indicator Symbol Letter C
1F1E9 ; fully-qualified # \u{1F1E9} E6.0 Regional Indicator Symbol Letter D
1F1EA ; fully-qualified # \u{1F1EA} E6.0 Regional Indicator Symbol Letter E
1F1EB ; fully-qualified # \u{1F1EB} E6.0 Regional Indicator Symbol Letter F
1F1EC ; fully-qualified # \u{1F1EC} E6.0 Regional Indicator Symbol Letter G
1F1ED ; fully-qualified # \u{1F1ED} E6.0 Regional Indicator Symbol Letter H
1F1EE ; fully-qualified # \u{1F1EE} E6.0 Regional Indicator Symbol Letter I
1F1EF ; fully-qualified # \u{1F1EF} E6.0 Regional Indicator Symbol Letter J
1F1F0 ; fully-qualified # \u{1F1F0} E6.0 Regional Indicator Symbol Letter K
1F1F1 ; fully-qualified # \u{1F1F1} E6.0 Regional Indicator Symbol Letter L
1F1F2 ; fully-qualified # \u{1F1F2} E6.0 Regional Indicator Symbol Letter M
1F1F3 ; fully-qualified # \u{1F1F3} E6.0 Regional Indicator Symbol Letter N
1F1F4 ; fully-qualified # \u{1F1F4} E6.0 Regional Indicator Symbol Letter O
1F1F5 ; fully-qualified # \u{1F1F5} E6.0 Regional Indicator Symbol Letter P
1F1F6 ; fully-qualified # \u{1F1F6} E6.0 Regional Indicator Symbol Letter Q
1F1F7 ; fully-qualified # \u{1F1F7} E6.0 Regional Indicator Symbol Letter R
1F1F8 ; fully-qualified # \u{1F1F8} E6.0 Regional Indicator Symbol Letter S
1F1F9 ; fully-qualified # \u{1F1F9} E6.0 Regional Indicator Symbol Letter T
1F1FA ; fully-qualified # \u{1F1FA} E6.0 Regional Indicator Symbol Letter U
1F1FB ; fully-qualified # \u{1F1FB} E6.0 Regional Indicator Symbol Letter V
1F1FC ; fully-qualified # \u{1F1FC} E6.0 Regional Indicator Symbol Letter W
1F1FD ; fully-qualified # \u{1F1FD} E6.0 Regional Indicator Symbol Letter X
1F1FE ; fully-qualified # \u{1F1FE} E6.0 Regional Indicator Symbol Letter Y
1F1FF ; fully-qualified # \u{1F1FF} E6.0 Regional Indicator Symbol Letter Z
";
