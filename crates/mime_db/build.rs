use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use unicase::UniCase;

#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Source {
    #[serde(rename = "iana")]
    IANA = 0,
    #[serde(rename = "apache")]
    Apache = 1,
    #[serde(rename = "nginx")]
    Nginx = 2,

    None = 3,
}

impl Default for Source {
    fn default() -> Self {
        Source::None
    }
}

#[derive(Debug, serde::Deserialize)]
struct MimeEntry {
    #[serde(default)]
    pub compressible: bool,

    #[serde(default)]
    pub extensions: Vec<String>,

    #[serde(default)]
    pub source: Source,
}

fn main() -> io::Result<()> {
    let mime_db = File::open("./mime-db/db.json")?;

    let db: HashMap<String, MimeEntry> = serde_json::from_reader(mime_db).unwrap();

    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("mime_db.rs");
    let mut file = BufWriter::new(File::create(&path)?);

    let mut mime_to_ext_map = phf_codegen::Map::new();
    let mut ext_to_mime_map = phf_codegen::Map::new();

    let mut ext_map: HashMap<&str, HashMap<&str, Source>> = HashMap::new();

    for (mime, entry) in db.iter() {
        let mut buf = format!(
            "MimeEntry {{ compressible: {}, extensions: &[",
            entry.compressible
        );

        for ext in &entry.extensions {
            buf += &format!("\"{}\", ", ext);

            ext_map.entry(ext).or_default().insert(mime, entry.source);
        }

        buf += "]}";

        mime_to_ext_map.entry(UniCase::new(mime), &buf);
    }

    for (ext, mapping_set) in ext_map.iter() {
        let mut mappings = mapping_set.iter().collect::<Vec<_>>();

        mappings.sort_by_key(|(_, source)| *source);

        let mut buf = "ExtEntry { types: &[".to_owned();

        for (mime, _) in mappings.iter() {
            buf += &format!("\"{}\", ", mime);
        }

        buf += "]}";

        ext_to_mime_map.entry(UniCase::new(*ext), &buf);
    }

    write!(
        &mut file,
        "static MIME_TO_EXT: phf::Map<UniCase<&'static str>, MimeEntry> = \n{};\n",
        mime_to_ext_map.build()
    )?;

    write!(
        &mut file,
        "static EXT_TO_MIME: phf::Map<UniCase<&'static str>, ExtEntry> = \n{};\n",
        ext_to_mime_map.build()
    )?;

    Ok(())
}
