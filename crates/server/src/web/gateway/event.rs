use std::ops::Deref;
use std::sync::Arc;

use miniz_oxide::deflate::core::TDEFLStatus;

use db::Snowflake;

use models::ReadyEvent;

use super::{
    msg::ServerMsg,
    socket::{GatewayMsgEncoding, GatewayQueryParams},
};

#[derive(Debug)]
pub struct CompressedEvent {
    pub uncompressed: Vec<u8>,
    pub compressed: Vec<u8>,
}

#[derive(Debug)]
pub struct EncodedEvent {
    pub json: CompressedEvent,
    pub msgpack: CompressedEvent,
}

#[derive(Debug)]
pub enum RawEvent {
    /// The socket doens't care about opaque events and should just send them
    ///
    /// This is a majority of events
    Opaque,

    Ready(Box<ReadyEvent>),
}

#[derive(Debug)]
pub struct EventInner {
    pub raw: RawEvent,
    pub encoded: EncodedEvent,
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
    pub fn new_opaque<S: serde::Serialize>(value: S) -> Result<Event, EncodingError> {
        let encoded = EncodedEvent::new(&value)?;
        let raw = RawEvent::Opaque;
        Ok(Event(Arc::new(EventInner { raw, encoded })))
    }

    pub fn new_ready(value: ReadyEvent) -> Result<Event, EncodingError> {
        // setup message
        let value = ServerMsg::new_ready(Box::new(value));

        // encode message
        let encoded = EncodedEvent::new(&value)?;

        // extract the boxed raw event again
        let raw = RawEvent::Ready(match value {
            ServerMsg::Ready { payload, .. } => payload.inner,
            _ => unsafe { std::hint::unreachable_unchecked() },
        });

        Ok(Event(Arc::new(EventInner { raw, encoded })))
    }
}

impl EncodedEvent {
    pub fn new<S: serde::Serialize>(value: &S) -> Result<Self, EncodingError> {
        let as_msgpack = rmp_serde::to_vec(value)?;
        let as_json = serde_json::to_vec(value)?;

        Ok(EncodedEvent {
            json: CompressedEvent::new(as_json)?,
            msgpack: CompressedEvent::new(as_msgpack)?,
        })
    }

    pub fn get(&self, params: GatewayQueryParams) -> &Vec<u8> {
        match params.encoding {
            GatewayMsgEncoding::Json => self.json.get(params.compress),
            GatewayMsgEncoding::MsgPack => self.msgpack.get(params.compress),
        }
    }
}

impl CompressedEvent {
    // TODO: Make async with `async-compression`?
    pub fn new(value: Vec<u8>) -> Result<Self, EncodingError> {
        let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&value, 7);

        //use flate2::{write::ZlibEncoder, Compression};
        //use std::io::Write;
        //
        //let mut encoder = ZlibEncoder::new(
        //    Vec::with_capacity((value.len() * 2) / 3),
        //    Compression::new(6),
        //);
        //encoder.write(&value)?;
        //let compressed = encoder.finish()?;

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
pub enum EncodingError {
    #[error("MsgPack Encoding Error: {0}")]
    MsgPackEncodingError(#[from] rmp_serde::encode::Error),

    #[error("Json Encoding Error: {0}")]
    JsonEncodingError(#[from] serde_json::Error),

    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
}