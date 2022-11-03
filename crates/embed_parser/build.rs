use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

static ATTRIBUTE_RE: &'static str = r#"[a-zA-Z_][0-9a-zA-Z\\-_]+\s*=\s*"(?:\\"|[^"])*[^\\]""#;
static URL: &'static str = r#"[^\s<]+[^<.,:;"')\]\s]"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(&env::var("OUT_DIR")?).join("codegen.rs");
    let mut file = BufWriter::new(File::create(&path)?);

    write!(file, "lazy_static::lazy_static! {{\n")?;
    write_regex(ATTRIBUTE_RE, "ATTRIBUTE_RE", &mut file)?;
    write_regex(URL, "URL", &mut file)?;
    write!(file, "\n}}")?;

    Ok(())
}

fn write_regex<W: Write>(re: &str, name: &str, mut out: W) -> Result<(), Box<dyn std::error::Error>> {
    use regex_automata::RegexBuilder;

    let re = RegexBuilder::new().minimize(true).build_with_size::<u16>(re)?;

    let mut size = 16;
    let mut forward = re.forward().to_bytes_native_endian()?;
    let mut reverse = re.reverse().to_bytes_native_endian()?;

    // try to shrink to u8s if possible
    if let (Ok(f), Ok(r)) = (re.forward().to_u8(), re.reverse().to_u8()) {
        size = 8;
        forward = f.to_bytes_native_endian()?;
        reverse = r.to_bytes_native_endian()?;
    }

    write!(
        out, "pub static ref {1}: Regex<DenseDFA<&'static [u{0}], u{0}>> = unsafe {{ Regex::from_dfas(DenseDFA::from_bytes(&{2:?}), DenseDFA::from_bytes(&{3:?})) }};",
        size, name, forward, reverse,
    )?;

    Ok(())
}
