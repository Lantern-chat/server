use std::borrow::Cow;
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use std::collections::hash_map::{Entry, HashMap};

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
    let mut emojis = HashMap::new();

    for line in src.lines() {
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

    let mut forms_to_full: HashMap<Cow<'_, str>, &str> = HashMap::new();
    let mut emoji_to_idx: HashMap<&str, usize> = HashMap::new();
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
