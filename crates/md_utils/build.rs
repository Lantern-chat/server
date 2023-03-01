use std::env;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(&env::var("OUT_DIR")?).join("codegen.rs");
    let mut file = BufWriter::new(File::create(path)?);

    regex_util::write_regex("NEWLINES", r#"(\r?\n){3,}"#, &mut file)?;

    Ok(())
}
