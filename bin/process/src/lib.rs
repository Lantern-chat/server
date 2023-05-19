#[macro_use]
extern crate serde;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EncodingFormat {
    Jpeg,
    Png,
    Avif,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Command {
    Initialize { width: u32, height: u32, max_pixels: u32 },
    ReadAndProcess { length: u64 },
    Encode { format: EncodingFormat, quality: u8 },
    Pause,
    Exit,
    Clear,
}

pub const HAS_ALPHA: u8 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessedResponse {
    pub preview: Option<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub flags: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Ready,
    Processed(ProcessedResponse),
    Encoded,
    Error(Error),
}

#[derive(Debug, Serialize, Deserialize, thiserror::Error)]
pub enum Error {
    #[error("IoError {0}")]
    IoError(IOErrorKind),
    #[error("InvalidImageFormat")]
    InvalidImageFormat,
    #[error("FileTooLarge")]
    FileTooLarge,
    #[error("ImageTooLarge")]
    ImageTooLarge,
    #[error("UnsupportedFormat")]
    UnsupportedFormat,
    #[error("EncodingError {0}")]
    EncodingError(String),
    #[error("DecodingError {0}")]
    DecodingError(String),
    #[error("SerializationError")]
    SerializationError,
    #[error("Other {0}")]
    Other(String),
}

#[cfg(feature = "binary")]
const _: () = {
    use processing::{process::ProcessingError, read_image::ImageReadError};

    impl From<bincode::Error> for Error {
        fn from(_: bincode::Error) -> Self {
            Error::SerializationError
        }
    }

    impl From<ProcessingError> for Error {
        fn from(err: ProcessingError) -> Self {
            match err {
                ProcessingError::IOError(err) => err.into(),
                ProcessingError::Other(err) => Error::Other(err),
            }
        }
    }

    impl From<processing::image::ImageError> for Error {
        fn from(err: processing::image::ImageError) -> Self {
            use processing::image::ImageError;

            match err {
                ImageError::Decoding(err) => Error::DecodingError(err.to_string()),
                ImageError::Encoding(err) => Error::EncodingError(err.to_string()),
                _ => Error::Other(err.to_string()),
            }
        }
    }

    impl From<ImageReadError> for Error {
        fn from(err: ImageReadError) -> Self {
            match err {
                ImageReadError::Io(err) => err.into(),
                ImageReadError::ImageTooLarge => Error::ImageTooLarge,
                ImageReadError::FileTooLarge => Error::FileTooLarge,
                ImageReadError::InvalidImageFormat => Error::InvalidImageFormat,
                ImageReadError::JpegDecodeError(_) | ImageReadError::PngDecodeError(_) => {
                    Error::DecodingError(err.to_string())
                }
                ImageReadError::Unsupported => Error::UnsupportedFormat,
                ImageReadError::Image(err) => err.into(),
            }
        }
    }

    use std::io;

    impl From<io::Error> for Error {
        fn from(err: io::Error) -> Self {
            Error::IoError(match err.kind() {
                io::ErrorKind::BrokenPipe => IOErrorKind::BrokenPipe,
                io::ErrorKind::InvalidData => IOErrorKind::InvalidData,
                io::ErrorKind::InvalidInput => IOErrorKind::InvalidInput,
                io::ErrorKind::UnexpectedEof => IOErrorKind::UnexpectedEof,
                io::ErrorKind::OutOfMemory => IOErrorKind::OutOfMemory,
                _ => IOErrorKind::Other(err.to_string()),
            })
        }
    }
};

#[derive(Debug, Serialize, Deserialize)]
pub enum IOErrorKind {
    BrokenPipe,
    InvalidData,
    InvalidInput,
    UnexpectedEof,
    OutOfMemory,
    Other(String),
}

use std::fmt;
impl fmt::Display for IOErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
