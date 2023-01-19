use std::ops::Deref;
use std::sync::Arc;

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
        let encoded = EncodedEvent::new(&msg)?;

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
    pub fn new<S: serde::Serialize>(value: &S) -> Result<Self, EventEncodingError> {
        let as_json = serde_json::to_vec(value)?;
        let as_cbor = {
            let mut buf = Vec::with_capacity(128);
            ciborium::ser::into_writer(value, &mut buf)?;
            buf
        };

        Ok(EncodedEvent {
            json: CompressedEvent::new(as_json)?,
            cbor: CompressedEvent::new(as_cbor)?,
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
    pub fn new(value: Vec<u8>) -> Result<Self, EventEncodingError> {
        let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&value, 7);

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

    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
}
