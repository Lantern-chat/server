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
