use std::str::FromStr;

use http::header::{HeaderValue, ToStrError};

use ftl::*;

use db::{
    schema::file::{File, FileFlags, Mime},
    Snowflake, SnowflakeExt
};

// TODO: Limit the number of files that can be pending at once, probably to 3
pub async fn post(route: Route<crate::ServerState>) -> Response {
    let upload_length = match route.parse_raw_header::<u32>("upload-length") {
        Some(Ok(Ok(upload_length))) => upload_length,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };

    if upload_length > route.state.config.max_upload_size {
        return super::REQUEST_ENTITY_TOO_LARGE.into_response();
    }

    let metadata = match Metadata::parse(route.raw_header("Upload-Metadata")) {
        Ok(metadata) => metadata,
        Err(e) => {
            return e
                .to_string()
                .with_status(StatusCode::BAD_REQUEST)
                .into_response()
        }
    };

    let file = File {
        id: Snowflake::now(),
        name: metadata.filename.to_owned(),
        preview: None,
        mime: Mime::from_str(metadata.filetype).ok(),
        size: upload_length,
        offset: 0,
        sha3: None,
        flags: FileFlags::empty(),
    };

    if let Err(e) = file.upsert(&route.state.db).await {
        log::error!("File Creation Error: {}", e);
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let mut res = Response::default();
    *res.status_mut() = StatusCode::CREATED;

    res.headers_mut().extend(
        super::TUS_HEADERS
            .iter()
            .map(|(k, v)| (k.clone(), v.clone())),
    );

    res.headers_mut().insert(
        "Location",
        HeaderValue::from_str(&format!("/api/v1/file/{}", file.id)).unwrap(),
    );

    res
}

#[derive(Clone, Copy)]
pub struct Metadata<'a> {
    filename: &'a str,
    filetype: &'a str,
}

#[derive(Debug, thiserror::Error)]
pub enum MetadataError {
    #[error("Missing Upload-Metadata header")]
    MissingHeader,

    #[error(transparent)]
    ToStrError(#[from] ToStrError),

    #[error("Missing filename")]
    MissingFilename,

    #[error("Missing filetype")]
    MissingFiletype,
}

impl<'a> Metadata<'a> {
    pub fn parse(header: Option<&'a HeaderValue>) -> Result<Metadata<'a>, MetadataError> {
        let metadata = match header {
            Some(header) => header.to_str()?,
            None => return Err(MetadataError::MissingHeader),
        };

        let mut filename = None;
        let mut filetype = None;

        for key_value in metadata.split(',') {
            let mut key_value_stream = key_value.split(' ');

            let key = key_value_stream.next();
            let val = key_value_stream.next();

            match (key, val) {
                (Some("filename"), Some(value)) if !value.is_empty() => filename = Some(value),
                (Some("filetype"), Some(value)) if !value.is_empty() => filetype = Some(value),
                _ => {}
            }
        }

        Ok(Metadata {
            filename: match filename {
                Some(v) => v,
                None => return Err(MetadataError::MissingFilename),
            },
            filetype: match filetype {
                Some(v) => v,
                None => return Err(MetadataError::MissingFiletype),
            },
        })
    }
}
