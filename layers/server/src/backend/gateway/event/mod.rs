use std::ops::Deref;
use triomphe::Arc;

use schema::Snowflake;

use sdk::api::gateway::{Encoding, GatewayQueryParams};

use sdk::models::gateway::message::ServerMsg;

pub mod internal;

/// Compressed and Uncompressed variants of a serialized event
#[derive(Debug)]
pub struct CompressedEvent {
    pub uncompressed: Vec<u8>,
    pub compressed: Vec<u8>,
}

/// Stores events in various formats that the gateway can send
#[derive(Debug)]
pub struct EncodedEvent {
    pub json: CompressedEvent,
    pub cbor: CompressedEvent,
}

pub use internal::InternalEvent;

/// An event that is intended to reach the external world
#[derive(Debug)]
pub struct ExternalEvent {
    pub msg: ServerMsg,
    pub encoded: EncodedEvent,
    pub room_id: Option<Snowflake>,
}

/// Actual event enum
#[derive(Debug)]
pub enum EventInner {
    Internal(InternalEvent),
    External(ExternalEvent),
}

/// `Arc<EventInner>` for efficient broadcasting of events
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct Event(Arc<EventInner>);

impl Deref for Event {
    type Target = EventInner;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Event {
    pub fn new(msg: ServerMsg, room_id: Option<Snowflake>) -> Result<Event, EventEncodingError> {
        Self::new_compressed(msg, room_id, 7)
    }

    pub fn new_compressed(
        msg: ServerMsg,
        room_id: Option<Snowflake>,
        compression_level: u8,
    ) -> Result<Event, EventEncodingError> {
        let encoded = EncodedEvent::new(&msg, compression_level)?;

        Ok(Event(Arc::new(EventInner::External(ExternalEvent {
            msg,
            encoded,
            room_id,
        }))))
    }

    pub fn internal(event: InternalEvent) -> Event {
        Event(Arc::new(EventInner::Internal(event)))
    }
}

impl EncodedEvent {
    pub fn new<S: serde::Serialize>(value: &S, compression_level: u8) -> Result<Self, EventEncodingError> {
        let as_json = serde_json::to_vec(value)?;
        let as_cbor = {
            let mut buf = Vec::with_capacity(128);
            ciborium::ser::into_writer(value, &mut buf)?;
            buf
        };

        Ok(EncodedEvent {
            json: CompressedEvent::new(as_json, compression_level)?,
            cbor: CompressedEvent::new(as_cbor, compression_level)?,
        })
    }

    pub fn get(&self, params: GatewayQueryParams) -> &Vec<u8> {
        match params.encoding {
            Encoding::JSON => self.json.get(params.compress),
            Encoding::CBOR => self.cbor.get(params.compress),
        }
    }
}

impl CompressedEvent {
    // TODO: Make async with `async-compression`?
    pub fn new(value: Vec<u8>, level: u8) -> Result<Self, EventEncodingError> {
        let compressed = thread_local_compress(&value, level)?;

        Ok(CompressedEvent {
            uncompressed: value,
            compressed,
        })
    }

    pub fn get(&self, compressed: bool) -> &Vec<u8> {
        match compressed {
            true => &self.compressed,
            false => &self.uncompressed,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EventEncodingError {
    #[error("Json Encoding Error: {0}")]
    JsonEncodingError(#[from] serde_json::Error),

    #[error("Cbor Encoding Error: {0}")]
    CborEncodingError(#[from] ciborium::ser::Error<std::io::Error>),

    #[error("Compression Error")]
    CompressionError,

    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
}

// Near exact copy of `miniz_oxide::deflate::compress_to_vec_inner` with thread-local compressor to reuse memory
fn thread_local_compress(input: &[u8], level: u8) -> Result<Vec<u8>, EventEncodingError> {
    use miniz_oxide::deflate::core::{
        compress, create_comp_flags_from_zip_params, CompressorOxide, TDEFLFlush, TDEFLStatus,
    };
    use std::cell::RefCell;

    thread_local! {
        static COMPRESSOR: RefCell<CompressorOxide> = RefCell::new(CompressorOxide::new(create_comp_flags_from_zip_params(7, 1, 0)));
    }

    COMPRESSOR.with(|compressor| {
        let Ok(mut compressor) = compressor.try_borrow_mut() else {
            return Err(EventEncodingError::CompressionError);
        };

        compressor.reset();
        compressor.set_compression_level_raw(level);

        let mut output = vec![0; std::cmp::max(input.len() / 2, 2)];

        let mut in_pos = 0;
        let mut out_pos = 0;

        loop {
            let (status, bytes_in, bytes_out) = compress(
                &mut compressor,
                &input[in_pos..],
                &mut output[out_pos..],
                TDEFLFlush::Finish,
            );

            out_pos += bytes_out;
            in_pos += bytes_in;

            match status {
                TDEFLStatus::Done => {
                    output.truncate(out_pos);
                    break;
                }
                TDEFLStatus::Okay => {
                    // We need more space, so resize the vector.
                    if output.len().saturating_sub(out_pos) < 30 {
                        output.resize(output.len() * 2, 0)
                    }
                }
                // Not supposed to happen unless there is a bug.
                _ => return Err(EventEncodingError::CompressionError),
            }
        }

        Ok(output)
    })
}
