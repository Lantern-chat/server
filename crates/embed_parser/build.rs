use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(&env::var("OUT_DIR")?).join("codegen.rs");
    let mut file = BufWriter::new(File::create(&path)?);

    regex_util::write_regex(
        "ATTRIBUTE_RE",
        r#"[a-zA-Z_][0-9a-zA-Z\\-_]+\s*=\s*"(?:\\"|[^"])*[^\\]""#,
        &mut file,
    )?;
    writeln!(file, "#[cfg(feature = \"msg\")]")?;
    regex_util::write_regex(
        "URL", //
        r#"[^\s<]+[^<.,:;"')\]\s]"#,
        &mut file,
    )?;

    Ok(())
}
