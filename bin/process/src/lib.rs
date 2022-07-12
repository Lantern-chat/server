#[macro_use]
extern crate serde;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum EncodingFormat {
    Jpeg,
    Png,
    Avif,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(C)]
pub enum Command {
    Initialize {
        width: u32,
        height: u32,
        max_pixels: u32,
    },
    ReadAndProcess {
        length: u64,
    },
    Encode {
        format: EncodingFormat,
        quality: u8,
    },
    Pause,
    Exit,
    Clear,
}

pub const HAS_ALPHA: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(C)]
pub enum Response {
    Ready,
    Processed {
        preview: Option<Vec<u8>>,
        width: u32,
        height: u32,
        flags: u8,
    },
    Encoded,
}
