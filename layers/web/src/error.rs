use std::{borrow::Cow, string::FromUtf8Error};

use db::pool::Error as DbError;
use ftl::{body::BodyDeserializeError, StatusCode};
use http::header::InvalidHeaderValue;
use sdk::{api::error::ApiErrorCode, models::UserPreferenceError};

lazy_static::lazy_static! {
    // 460 Checksum Mismatch
    pub static ref CHECKSUM_MISMATCH: StatusCode = StatusCode::from_u16(460).unwrap();

    // 413 Request Entity Too Large
    pub static ref REQUEST_ENTITY_TOO_LARGE: StatusCode = StatusCode::from_u16(413).unwrap();
}

#[derive(Debug, thiserror::Error)]
pub enum WebError {
    // FATAL ERRORS
    #[error("Semaphore Error: {0}")]
    SemaphoreError(#[from] tokio::sync::AcquireError),

    #[error("Parse Error {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("XML Deserialize Error {0}")]
    XMLError(#[from] quick_xml::de::DeError),

    #[error("IO Error: {0}")]
    IOError(std::io::Error),
    #[error("Invalid Header Value: {0}")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
    #[error("Internal Error: {0}")]
    InternalError(String),
    #[error("Internal Error: {0}")]
    InternalErrorStatic(&'static str),
    #[error("Unimplemented")]
    Unimplemented,

    #[error("UTF8 Parse Error: {0}")]
    Utf8ParseError(#[from] FromUtf8Error),

    // NON-FATAL ERRORS
    #[error("Missing Upload-Metadata Header")]
    MissingUploadMetadataHeader,

    #[error("Missing Authorization Header")]
    MissingAuthorizationHeader,

    #[error("Invalid Auth Format")]
    InvalidAuthFormat,

    #[error("Header Parse Error")]
    HeaderParseError(#[from] http::header::ToStrError),

    #[error("Not Found")]
    NotFound,

    #[error("Bad Request")]
    BadRequest,

    #[error("Upload Error")]
    UploadError,

    #[error("Upload Conflict")]
    Conflict,

    #[error("Auth Token Error: {0}")]
    AuthTokenError(#[from] schema::auth::AuthTokenError),

    #[error("Base-64 Decode Error: {0}")]
    Base64DecodeError(#[from] base64::DecodeError),

    #[error("Body Deserialization Error: {0}")]
    BodyDeserializeError(#[from] BodyDeserializeError),

    #[error("Query Parse Error: {0}")]
    QueryParseError(#[from] serde_urlencoded::de::Error),

    #[error("Checksum Mismatch")]
    ChecksumMismatch,

    #[error("Request Entity Too Large")]
    RequestEntityTooLarge,

    #[error("Mime Parse Error: {0}")]
    MimeParseError(#[from] mime::FromStrError),

    #[error("Unauthorized")]
    Unauthorized,
}

impl From<std::io::Error> for WebError {
    fn from(err: std::io::Error) -> WebError {
        if err.kind() == std::io::ErrorKind::NotFound {
            WebError::NotFound
        } else {
            WebError::IOError(err)
        }
    }
}

/*
impl Error {
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            Error::DbError(_)
                | Error::JoinError(_)
                | Error::SemaphoreError(_)
                | Error::HashError(_)
                | Error::JsonError(_)
                | Error::XMLError(_)
                | Error::EventEncodingError(_)
                | Error::IOError(_)
                | Error::InternalError(_)
                | Error::InternalErrorStatic(_)
                | Error::InvalidHeaderValue(_)
                | Error::RequestError(_)
                | Error::Unimplemented
        )
    }

    pub fn http_status(&self) -> StatusCode {
        if self.is_fatal() {
            return StatusCode::INTERNAL_SERVER_ERROR;
        }

        match self {
            Error::NoSession
            | Error::InvalidCredentials
            | Error::TOTPRequired
            | Error::Unauthorized
            | Error::InvalidCaptcha(_) => StatusCode::UNAUTHORIZED,
            Error::TemporarilyDisabled => StatusCode::FORBIDDEN,
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::BadRequest => StatusCode::BAD_REQUEST,
            Error::AlreadyExists => StatusCode::CONFLICT,
            Error::MissingAuthorizationHeader
            | Error::MissingUploadMetadataHeader
            | Error::HeaderParseError(_)
            | Error::AuthTokenError(_)
            | Error::Base64DecodeError(_)
            | Error::Base85DecodeError(_)
            | Error::BodyDeserializeError(_)
            | Error::JsonError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Error::ChecksumMismatch => *CHECKSUM_MISMATCH,
            Error::RequestEntityTooLarge => *REQUEST_ENTITY_TOO_LARGE,
            Error::Conflict => StatusCode::CONFLICT,
            _ => StatusCode::BAD_REQUEST,
        }
    }

    #[rustfmt::skip]
    pub fn to_apierror(&self) -> ApiErrorCode {
        match *self {
            Error::DbError(_)               => ApiErrorCode::DbError,
            Error::JoinError(_)             => ApiErrorCode::JoinError,
            Error::SemaphoreError(_)        => ApiErrorCode::SemaphoreError,
            Error::HashError(_)             => ApiErrorCode::HashError,
            Error::JsonError(_)             => ApiErrorCode::JsonError,
            Error::EventEncodingError(_)    => ApiErrorCode::EventEncodingError,
            Error::InternalError(_)         => ApiErrorCode::InternalError,
            Error::InternalErrorStatic(_)   => ApiErrorCode::InternalErrorStatic,
            Error::Utf8ParseError(_)        => ApiErrorCode::Utf8ParseError,
            Error::IOError(_)               => ApiErrorCode::IOError,
            Error::InvalidHeaderValue(_)    => ApiErrorCode::InvalidHeaderValue,
            Error::XMLError(_)              => ApiErrorCode::XMLError,
            Error::RequestError(_)          => ApiErrorCode::RequestError,
            Error::Unimplemented            => ApiErrorCode::Unimplemented,

            Error::AlreadyExists            => ApiErrorCode::AlreadyExists,
            Error::UsernameUnavailable      => ApiErrorCode::UsernameUnavailable,
            Error::InvalidEmail             => ApiErrorCode::InvalidEmail,
            Error::InvalidUsername          => ApiErrorCode::InvalidUsername,
            Error::InvalidPassword          => ApiErrorCode::InvalidPassword,
            Error::InvalidCredentials       => ApiErrorCode::InvalidCredentials,
            Error::InsufficientAge          => ApiErrorCode::InsufficientAge,
            Error::InvalidDate              => ApiErrorCode::InvalidDate,
            Error::InvalidContent           => ApiErrorCode::InvalidContent,
            Error::InvalidName              => ApiErrorCode::InvalidName,
            Error::InvalidTopic             => ApiErrorCode::InvalidTopic,
            Error::MissingUploadMetadataHeader  => ApiErrorCode::MissingUploadMetadataHeader,
            Error::MissingAuthorizationHeader   => ApiErrorCode::MissingAuthorizationHeader,
            Error::NoSession                => ApiErrorCode::NoSession,
            Error::InvalidAuthFormat        => ApiErrorCode::InvalidAuthFormat,
            Error::HeaderParseError(_)      => ApiErrorCode::HeaderParseError,
            Error::MissingFilename          => ApiErrorCode::MissingFilename,
            Error::MissingMime              => ApiErrorCode::MissingMime,
            Error::AuthTokenError(_)        => ApiErrorCode::AuthTokenError,
            Error::Base64DecodeError(_)     => ApiErrorCode::Base64DecodeError,
            Error::BodyDeserializeError(_)  => ApiErrorCode::BodyDeserializeError,
            Error::QueryParseError(_)       => ApiErrorCode::QueryParseError,
            Error::UploadError              => ApiErrorCode::UploadError,
            Error::InvalidPreview           => ApiErrorCode::InvalidPreview,
            Error::MimeParseError(_)        => ApiErrorCode::MimeParseError,
            Error::InvalidImageFormat       => ApiErrorCode::InvalidImageFormat,
            Error::TOTPRequired             => ApiErrorCode::TOTPRequired,
            Error::InvalidPreferences(_)    => ApiErrorCode::InvalidPreferences,
            Error::TemporarilyDisabled      => ApiErrorCode::TemporarilyDisabled,
            Error::InvalidCaptcha(_)        => ApiErrorCode::InvalidCaptcha,
            Error::Base85DecodeError(_)     => ApiErrorCode::Base85DecodeError,


            // HTTP-like error codes
            Error::BadRequest               => ApiErrorCode::BadRequest,
            Error::Unauthorized             => ApiErrorCode::Unauthorized,
            Error::NotFound                 => ApiErrorCode::NotFound,
            Error::Conflict                 => ApiErrorCode::Conflict,
            Error::RequestEntityTooLarge    => ApiErrorCode::RequestEntityTooLarge,
            Error::ChecksumMismatch         => ApiErrorCode::ChecksumMismatch,
        }
    }

    #[inline]
    pub fn code(&self) -> u16 {
        self.to_apierror() as u16
    }

    pub fn format(&self) -> Cow<'static, str> {
        Cow::Borrowed(match self {
            // TODO: at least say if it's a database error, for now
            Error::DbError(_) => "Database Error",
            Error::AuthTokenError(_) => "Auth Token Parse Error",
            Error::Base64DecodeError(_) => "Base64 Decode Error",
            Error::Base85DecodeError(_) => "Base85 Decode Error",
            Error::IOError(_) => "IO Error",
            Error::InvalidHeaderValue(_) => "Invalid Header Value",

            _ if self.is_fatal() => "Internal Server Error",
            _ => return self.to_string().into(),
        })
    }
}
 */
