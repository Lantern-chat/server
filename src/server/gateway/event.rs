use std::sync::Arc;

pub struct CompressedEvent {
    decompressed: Vec<u8>,
    compressed: Vec<u8>,
}

pub struct EncodedEvent {
    json: CompressedEvent,
    msgpack: CompressedEvent,
}

pub enum RawEvent {
    /// The socket doens't care about opaque events and should just send them
    ///
    /// This is a majority of events
    Opaque,
}

pub struct EventInner {
    pub raw: RawEvent,
    pub encoded: EncodedEvent,
}

#[derive(Clone)]
pub struct Event(Arc<EventInner>);

impl EncodedEvent {
    pub fn new<S: serde::Serialize>(value: &S) -> Result<Self, EncodingError> {
        let as_msgpack = rmp_serde::to_vec(value)?;
        let as_json = serde_json::to_vec(value)?;

        Ok(EncodedEvent {
            json: CompressedEvent::new(as_json)?,
            msgpack: CompressedEvent::new(as_msgpack)?,
        })
    }
}

impl CompressedEvent {
    pub fn new(value: Vec<u8>) -> Result<Self, EncodingError> {
        use flate2::{write::ZlibEncoder, Compression};
        use std::io::Write;

        let mut encoder = ZlibEncoder::new(
            Vec::with_capacity((value.len() * 2) / 3),
            Compression::new(6),
        );
        encoder.write(&value)?;
        let compressed = encoder.finish()?;

        Ok(CompressedEvent {
            decompressed: value,
            compressed,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EncodingError {
    #[error("MsgPack Encoding Error: {0}")]
    MsgPackEncodingError(#[from] rmp_serde::encode::Error),

    #[error("Json Encoding Error: {0}")]
    JsonEncodingError(#[from] serde_json::Error),

    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
}
