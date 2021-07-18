use unicase::UniCase;

#[derive(Debug, Clone, Copy)]
pub struct MimeEntry {
    compressible: bool,
    extensions: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
pub struct ExtEntry {
    types: &'static [&'static str],
}

include!(concat!(env!("OUT_DIR"), "/mime_db.rs"));

pub fn lookup_ext(ext: &str) -> Option<&ExtEntry> {
    EXT_TO_MIME.get(&UniCase::new(ext))
}

pub fn lookup_mime(mime: &str) -> Option<&MimeEntry> {
    MIME_TO_EXT.get(&UniCase::new(mime))
}

#[inline]
pub fn lookup_mime_from_ext(ext: &str) -> Option<&MimeEntry> {
    let entry = lookup_ext(ext)?;

    if entry.types.is_empty() {
        return None;
    }

    // Lookup IANA entry
    lookup_mime(entry.types[0])
}
