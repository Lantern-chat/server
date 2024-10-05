use once_cell::sync::OnceCell;
use std::ops::Deref;
use thin_vec::ThinVec;
use triomphe::Arc;

use crate::prelude::*;

use sdk::api::gateway::{Encoding, GatewayQueryParams};
use sdk::models::gateway::message::ServerMsg;

pub mod internal;
pub use internal::InternalEvent;

use std::sync::LazyLock;

pub static HELLO_EVENT: LazyLock<Event> = LazyLock::new(|| Event::new_compressed(ServerMsg::new_hello(sdk::models::events::Hello::default()), None, 10).unwrap());
pub static HEARTBEAT_ACK: LazyLock<Event> = LazyLock::new(|| Event::new_compressed(ServerMsg::new_heartbeat_ack(), None, 10).unwrap());
pub static INVALID_SESSION: LazyLock<Event> = LazyLock::new(|| Event::new_compressed(ServerMsg::new_invalid_session(), None, 10).unwrap());

#[derive(Debug, thiserror::Error)]
pub enum EventEncodingError {
    #[error("Json Encoding Error: {0}")]
    JsonEncodingError(#[from] serde_json::Error),

    #[error("Cbor Encoding Error: {0}")]
    CborEncodingError(#[from] ciborium::ser::Error<std::io::Error>),

    #[error("Compression Error: {0}")]
    CompressionError(#[from] DeflateError),

    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct CompressedEvent {
    pub uncompressed: ThinVec<u8>,
    pub compressed: ThinVec<u8>,
}

#[derive(Debug)]
pub struct EncodedEvent {
    pub json: CompressedEvent,
    pub cbor: CompressedEvent,
}

/// An event that is intended to reach the external world
#[derive(Debug)]
pub struct ExternalEvent {
    pub msg: ServerMsg,
    pub room_id: Option<RoomId>,

    // TODO: Replace with std OnceLock when get_or_try_init is stabilized
    // https://github.com/rust-lang/rust/issues/109737
    pub encoded: OnceCell<EncodedEvent>,
}

/// Actual event enum
#[derive(Debug)]
pub enum EventInner {
    Internal(InternalEvent),
    External(ExternalEvent),
}

/// `Arc<EventInner>` for efficient broadcasting of events
#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct Event(Arc<EventInner>);

impl Deref for Event {
    type Target = EventInner;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

use util::zlib::{deflate, DeflateError};

impl CompressedEvent {
    fn new(level: u8, uncompressed: ThinVec<u8>) -> Result<Self, EventEncodingError> {
        Ok(CompressedEvent {
            compressed: deflate(&uncompressed, level)?,
            uncompressed,
        })
    }

    pub fn get(&self, compressed: bool) -> &[u8] {
        match compressed {
            true => &self.compressed,
            false => &self.uncompressed,
        }
    }
}

impl EncodedEvent {
    pub fn new<S: serde::Serialize>(value: &S, compression_level: u8) -> Result<Self, EventEncodingError> {
        Ok(EncodedEvent {
            json: CompressedEvent::new(compression_level, {
                let mut buf = ThinVec::with_capacity(128);
                serde_json::to_writer(&mut buf, value)?;
                buf
            })?,
            cbor: CompressedEvent::new(compression_level, {
                let mut buf = ThinVec::with_capacity(128);
                ciborium::ser::into_writer(value, &mut buf)?;
                buf
            })?,
        })
    }

    pub fn get(&self, params: GatewayQueryParams) -> &[u8] {
        match params.encoding {
            Encoding::JSON => self.json.get(params.compress),
            Encoding::CBOR => self.cbor.get(params.compress),
        }
    }
}

impl ExternalEvent {
    /// Attempts to get the encoded event. If the event has not been encoded yet, it will be encoded now using the
    /// specified compression level. The compression level has no effect if the event has already been encoded.
    ///
    /// This method is thread-safe and will only encode the event once.
    pub fn get_encoded(&self, level: u8) -> Result<&EncodedEvent, EventEncodingError> {
        self.encoded.get_or_try_init(|| EncodedEvent::new(&self.msg, level))
    }
}

impl Event {
    pub const DEFAULT_COMPRESSION_LEVEL: u8 = 7;

    /// Constructs a new external event, but does not encode it yet.
    pub fn new(msg: ServerMsg, room_id: Option<RoomId>) -> Event {
        Event(Arc::new(EventInner::External(ExternalEvent {
            msg,
            encoded: OnceCell::new(),
            room_id,
        })))
    }

    pub fn new_compressed(msg: ServerMsg, room_id: Option<RoomId>, level: u8) -> Result<Event, EventEncodingError> {
        let encoded = EncodedEvent::new(&msg, level)?;

        Ok(Event(Arc::new(EventInner::External(ExternalEvent {
            msg,
            encoded: OnceCell::with_value(encoded),
            room_id,
        }))))
    }

    pub fn internal(event: InternalEvent) -> Event {
        Event(Arc::new(EventInner::Internal(event)))
    }
}
