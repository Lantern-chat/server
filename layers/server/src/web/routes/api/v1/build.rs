#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct BuildInfo {
    pub server: &'static str,
    pub target: &'static str,
    pub debug: bool,
    pub time: &'static str,
    //pub commit: Option<&'static str>,
    //pub authors: &'static str,
}

pub const BUILD_INFO: BuildInfo = BuildInfo {
    server: crate::built::PKG_VERSION,
    target: crate::built::TARGET,
    debug: crate::built::DEBUG,
    time: crate::built::BUILT_TIME_UTC,
    //commit: crate::built::GIT_VERSION,
    //authors: crate::built::PKG_AUTHORS,
};

use bytes::Bytes;
use headers::ContentType;

use crate::web::encoding::EncodingQuery;

use super::*;

pub fn build(route: Route<ServerState>) -> WebResult {
    // TODO: Come up with a way to cache serde stuff
    Ok(WebResponse::new(&BUILD_INFO))

    // lazy_static::lazy_static! {
    //     static ref JSON_BUILD_INFO: String = serde_json::to_string(&BUILD_INFO).unwrap();
    //     //static ref MSGPACK_BUILD_INFO: Bytes = rmp_serde::to_vec(&BUILD_INFO).unwrap().into();
    // }

    // Ok(match route.query::<EncodingQuery>() {
    //     //Some(Ok(EncodingQuery {
    //     //    encoding: Encoding::MsgPack,
    //     //})) => bytes_as_msgpack(MSGPACK_BUILD_INFO.clone()),
    //     _ => JSON_BUILD_INFO
    //         .as_str()
    //         .with_header(ContentType::json())
    //         .into_response(),
    // })
}
