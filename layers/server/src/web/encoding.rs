use bytes::Bytes;
use ftl::*;
use headers::ContentType;

use sdk::driver::Encoding;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncodingQuery {
    #[serde(default)]
    #[serde(alias = "e")]
    pub encoding: Encoding,
}

//pub fn bytes_as_msgpack(bytes: Bytes) -> Response {
//    hyper::Body::from(bytes)
//        .with_header(ContentType::from(mime::APPLICATION_MSGPACK))
//        .into_response()
//}

pub fn bytes_as_json(bytes: Bytes) -> Response {
    hyper::Body::from(bytes).with_header(ContentType::json()).into_response()
}
