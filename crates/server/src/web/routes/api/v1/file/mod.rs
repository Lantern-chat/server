use http::{HeaderMap, HeaderValue, Method, StatusCode};

use ftl::*;

// https://tus.io/protocols/resumable-upload.html

lazy_static::lazy_static! {
    pub static ref TUS_HEADERS: HeaderMap<HeaderValue> = {
        let mut headers = HeaderMap::new();

        headers.insert("Tus-Resumable", HeaderValue::from_static("1.0.0"));
        headers.insert("Tus-Version", HeaderValue::from_static("1.0.0"));
        //headers.insert("Tus-Extension", HeaderValue::from_static("creation,expiration,termination"));
        headers.insert("Tus-Extension", HeaderValue::from_static("creation,expiration,checksum,termination"));
        headers.insert("Tus-Checksum-Algorithm", HeaderValue::from_static("crc32"));

        headers
    };

    // 460 Checksum Mismatch
    pub static ref CHECKSUM_MISMATCH: StatusCode = StatusCode::from_u16(460).unwrap();

    // 413 Request Entity Too Large
    pub static ref REQUEST_ENTITY_TOO_LARGE: StatusCode = StatusCode::from_u16(413).unwrap();
}

use crate::{web::routes::api::ApiError, ServerState};

pub mod options;
pub mod post;

pub async fn file(mut route: Route<ServerState>) -> Response {
    match route.next().method_segment() {
        (&Method::OPTIONS, End) => options::options(route),

        (&Method::POST, End) => post::post(route).await,

        _ => ApiError::not_found().into_response(),
    }
}
