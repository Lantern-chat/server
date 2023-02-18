use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(&env::var("OUT_DIR")?).join("codegen.rs");
    let mut file = BufWriter::new(File::create(&path)?);

    regex_util::write_regex(
        "ATTRIBUTE_RE", // helps with splitting name="value"
        r#"[a-zA-Z_][0-9a-zA-Z\-_]+\s*=\s*"(?:\\"|[^"])*[^\\]""#,
        &mut file,
    )?;
    writeln!(file, "#[cfg(feature = \"msg\")]")?;
    regex_util::write_regex(
        "URL", // helps URL parsing in messages
        r#"[^\s<]+[^<.,:;"')\]\s]"#,
        &mut file,
    )?;
    regex_util::write_regex(
        "META_TAGS", // identifies HTML tags valid for metadata
        r#"<(meta\x20|title>|link\x20)"#,
        &mut file,
    )?;
    regex_util::write_regex(
        "ADULT_RATING", // case-insensitive rating
        r#"(?i)adult|mature|RTA\-5042\-1996\-1400\-1577\-RTA"#,
        &mut file,
    )?;

    Ok(())
}
