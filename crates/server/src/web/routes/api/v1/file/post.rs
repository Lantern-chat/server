use ftl::*;

use schema::Snowflake;

use crate::{
    ctrl::Error,
    web::{auth::Authorization, routes::api::ApiError},
    ServerState,
};

pub async fn post(route: Route<ServerState>, auth: Authorization) -> Response {
    let upload_length = match route.parse_raw_header::<i32>("upload-length") {
        Some(Ok(Ok(upload_length))) => upload_length,
        _ => return ApiError::bad_request().into_response(),
    };

    if upload_length > route.state.config.max_upload_size {
        return ApiError::err(Error::RequestEntityTooLarge).into_response();
    }

    let metadata = match Metadata::parse(route.raw_header("Upload-Metadata")) {
        Ok(metadata) => metadata,
        Err(e) => return ApiError::err(e).into_response(),
    };

    match crate::ctrl::file::post::post_file(route.state.clone(), auth, upload_length, metadata).await {
        Err(e) => ApiError::err(e).into_response(),
        Ok(file_id) => {
            let mut res = Response::default();
            *res.status_mut() = StatusCode::CREATED;

            res.headers_mut()
                .extend(super::TUS_HEADERS.iter().map(|(k, v)| (k.clone(), v.clone())));

            res.headers_mut().insert(
                "Location",
                HeaderValue::from_str(&format!("/api/v1/file/{}", file_id)).unwrap(),
            );

            res
        }
    }
}

use http::header::HeaderValue;
use std::str::FromStr;

/// Base64-encoded metadata fields
#[derive(Clone, Copy)]
pub struct Metadata<'a> {
    pub filename: &'a str,
    pub mime: Option<&'a str>,
    pub preview: Option<&'a str>,
}

impl<'a> Metadata<'a> {
    pub fn parse(header: Option<&'a HeaderValue>) -> Result<Metadata<'a>, Error> {
        let metadata = match header {
            Some(header) => header.to_str()?,
            None => return Err(Error::MissingUploadMetadataHeader),
        };

        let mut filename = None;
        let mut mime = None;
        let mut preview = None;

        for key_value in metadata.split(',') {
            let mut key_value_stream = key_value.split(' ').map(str::trim);

            let key = key_value_stream.next();
            let val = key_value_stream.next();

            match (key, val) {
                (Some("filename"), Some(value)) if !value.is_empty() => filename = Some(value),
                (Some("mime"), Some(value)) if !value.is_empty() => mime = Some(value),
                (Some("preview"), Some(value)) if !value.is_empty() => preview = Some(value),
                _ => {}
            }
        }

        Ok(Metadata {
            filename: match filename {
                Some(v) => v,
                None => return Err(Error::MissingFilename),
            },
            mime,
            preview,
        })
    }
}
