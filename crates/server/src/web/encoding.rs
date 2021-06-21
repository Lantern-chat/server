use bytes::Bytes;
use ftl::*;
use headers::ContentType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Encoding {
    /// Textual JSON, simple.
    Json,

    /// Binary MessagePack (smaller, slower to encode/decode in browser)
    ///
    /// This is recommended when you have access to natively compiled MsgPack libraries
    MsgPack,
}

impl Default for Encoding {
    fn default() -> Self {
        Encoding::Json
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncodingQuery {
    #[serde(default)]
    pub encoding: Encoding,
}

pub fn bytes_as_msgpack(bytes: Bytes) -> Response {
    hyper::Body::from(bytes)
        .with_header(ContentType::from(mime::APPLICATION_MSGPACK))
        .into_response()
}
