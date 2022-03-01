use std::ops::Deref;
use std::sync::Arc;

use miniz_oxide::deflate::core::TDEFLStatus;

use schema::Snowflake;

use crate::web::encoding::Encoding;

use super::socket::GatewayQueryParams;

use sdk::models::gateway::message::ServerMsg;

#[derive(Debug)]
pub struct CompressedEvent {
    pub uncompressed: Vec<u8>,
    pub compressed: Vec<u8>,
}

#[derive(Debug)]
pub struct EncodedEvent {
    pub json: CompressedEvent,
    pub cbor: CompressedEvent,
}

#[derive(Debug)]
pub struct EventInner {
    pub msg: ServerMsg,
    pub encoded: EncodedEvent,
    pub room_id: Option<Snowflake>,
}

#[derive(Debug, Clone)]
pub struct Event(Arc<EventInner>);

impl Deref for Event {
    type Target = EventInner;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl Event {
    pub fn new(msg: ServerMsg, room_id: Option<Snowflake>) -> Result<Event, EventEncodingError> {
        let encoded = EncodedEvent::new(&msg)?;

        Ok(Event(Arc::new(EventInner {
            msg,
            encoded,
            room_id,
        })))
    }
}

impl EncodedEvent {
    pub fn new<S: serde::Serialize>(value: &S) -> Result<Self, EventEncodingError> {
        let as_json = serde_json::to_vec(value)?;
        let as_cbor = {
            let mut buf = Vec::with_capacity(std::mem::size_of_val(value));
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
            Encoding::Json => self.json.get(params.compress),
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
