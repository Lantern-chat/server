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
