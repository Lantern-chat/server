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

/// https://en.wikipedia.org/wiki/List_of_file_signatures
pub fn from_prefix(bytes: &[u8]) -> Option<(&str, Option<&MimeEntry>)> {
    static MAGIC_BYTES: &[(usize, &[u8], &str)] = &[
        (0, b"\x89PNG\r\n\x1a\n", "image/png"),
        (0, &[0xff, 0xd8, 0xff], "image/jpeg"),
        (0, &[0xCF, 0x84, 0x01], "image/jpeg"),
        (0, b"GIF89a", "image/gif"),
        (0, b"GIF87a", "image/gif"),
        (0, b"MM\x00*", "image/tiff"),
        (0, b"II*\x00", "image/tiff"),
        (0, b"DDS ", "image/vnd.ms-dds"),
        (0, b"BM", "image/bmp"),
        (0, &[0, 0, 1, 0], "image/x-icon"),
        (0, b"#?RADIANCE", "image/vnd.radiance"),
        (0, b"P1", "image/x-portable-anymap"),
        (0, b"P2", "image/x-portable-anymap"),
        (0, b"P3", "image/x-portable-anymap"),
        (0, b"P4", "image/x-portable-anymap"),
        (0, b"P5", "image/x-portable-anymap"),
        (0, b"P6", "image/x-portable-anymap"),
        (0, b"P7", "image/x-portable-anymap"),
        (0, b"farbfeld", "image/x-farbfeld"),
        (0, b"\0\0\0 ftypavif", "image/avif"),
        (0, &[0x76, 0x2f, 0x31, 0x01], "image/aces"), // = &exr::meta::magic_number::BYTES
        (0, &[0x38, 0x42, 0x50, 0x53], "image/vnd.adobe.photoshop"),
        (0, &[0x25, 0x50, 0x44, 0x46, 0x2D], "application/pdf"),
        (0, &[0x4F, 0x67, 0x67, 0x53], "audio/ogg"),
        (0, &[0xFF, 0xFB], "audio/mp3"),
        (0, &[0xFF, 0xF3], "audio/mp3"),
        (0, &[0xFF, 0xF2], "audio/mp3"),
        (0, &[0x49, 0x44, 0x33], "audio/mp3"),
        (0, &[0x66, 0x4C, 0x61, 0x43], "audio/x-flac"),
        (
            0,
            &[
                0x00, 0x00, 0x00, 0x0C, 0x4A, 0x58, 0x4C, 0x20, 0x0D, 0x0A, 0x87, 0x0A,
            ],
            "image/jxl",
        ),
        (0, &[0x4D, 0x54, 0x68, 0x64], "audio/midi"),
        (
            0,
            &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1],
            "application/msword",
        ),
        (0, &[0x1F, 0x8B], "application/gzip"),
        (
            257,
            &[0x75, 0x73, 0x74, 0x61, 0x72, 0x00, 0x30, 0x30],
            "application/tar",
        ),
        (
            257,
            &[0x75, 0x73, 0x74, 0x61, 0x72, 0x20, 0x20, 0x00],
            "application/tar",
        ),
        (
            0,
            &[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C],
            "application/x-7z-compressed",
        ),
        (0, &[0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00], "application/x-xz"),
        (0, &[0x46, 0x4C, 0x49, 0x46], "image/flif"),
        (0, &[0x1A, 0x45, 0xDF, 0xA3], "video/x-matroska"),
        (0, &[0x47], "video/mpeg"),
        (4, &[0x66, 0x74, 0x79, 0x70, 0x69, 0x73, 0x6F, 0x6D], "video/mp4"),
        (0, &[0x78, 0x01], "application/z-lib"),
        (0, &[0x78, 0x5E], "application/z-lib"),
        (0, &[0x78, 0x9C], "application/z-lib"),
        (0, &[0x78, 0xDA], "application/z-lib"),
        (0, &[0x78, 0x20], "application/z-lib"),
        (0, &[0x78, 0x7D], "application/z-lib"),
        (0, &[0x78, 0xBB], "application/z-lib"),
        (0, &[0x78, 0xF9], "application/z-lib"),
        (
            0,
            &[0x42, 0x4C, 0x45, 0x4E, 0x44, 0x45, 0x52],
            "application/x-blend",
        ),
        (0, &[0x46, 0x4C, 0x56], "video/x-flv"),
        (0, &[0x4D, 0x53, 0x43, 0x46], "application/vnd.ms-cab-compressed"),
        (
            0,
            &[
                0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11, 0xA6, 0xD9, 0x00, 0xAA, 0x00, 0x62, 0xCE,
                0x6C,
            ],
            "video/x-ms-wmv",
        ),
        (
            0,
            &[
                0x53, 0x49, 0x4D, 0x50, 0x4C, 0x45, 0x20, 0x20, 0x3D, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
                0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x54,
            ],
            "image/fits",
        ),
    ];

    const RIFFS: &[(&[u8], &str)] = &[
        (&[0x57, 0x45, 0x42, 0x50], "image/webp"),
        (&[0x57, 0x41, 0x56, 0x45], "audio/wav"),
        (&[0x41, 0x56, 0x49, 0x20], "video/x-msvideo"),
        (&[0x43, 0x44, 0x44, 0x41], "audio/cda"),
    ];

    for (offset, prefix, mime) in MAGIC_BYTES {
        if bytes.len() > *offset && bytes[*offset..].starts_with(prefix) {
            return Some((*mime, lookup_mime(mime)));
        }

        if bytes.starts_with(b"RIFF") && bytes.len() >= 12 {
            let bytes = &bytes[4..];
            for (prefix, mime) in RIFFS {
                if bytes.starts_with(prefix) {
                    return Some((*mime, lookup_mime(mime)));
                }
            }
        }
    }

    None
}
