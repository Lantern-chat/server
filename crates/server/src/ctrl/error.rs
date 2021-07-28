use std::{borrow::Cow, string::FromUtf8Error};

use db::pool::Error as DbError;
use ftl::{body::BodyDeserializeError, StatusCode};

use crate::web::gateway::event::EncodingError;

lazy_static::lazy_static! {
    // 460 Checksum Mismatch
    pub static ref CHECKSUM_MISMATCH: StatusCode = StatusCode::from_u16(460).unwrap();

    // 413 Request Entity Too Large
    pub static ref REQUEST_ENTITY_TOO_LARGE: StatusCode = StatusCode::from_u16(413).unwrap();
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // FATAL ERRORS
    // TODO: Add backtraces when https://github.com/dtolnay/thiserror/pull/131 lands
    #[error("Database Error {0}")]
    DbError(#[from] DbError),
    #[error("Join Error {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Semaphore Error: {0}")]
    SemaphoreError(#[from] tokio::sync::AcquireError),
    #[error("Password Hash Error {0}")]
    HashError(#[from] argon2::Error),
    #[error("Parse Error {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Encoding Error {0}")]
    EncodingError(#[from] EncodingError),
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Internal Error: {0}")]
    InternalError(String),
    #[error("Internal Error: {0}")]
    InternalErrorStatic(&'static str),

    #[error("UTF8 Parse Error: {0}")]
    Utf8ParseError(#[from] FromUtf8Error),

    // NON-FATAL ERRORS
    #[error("Already Exists")]
    AlreadyExists,

    #[error("Username Unavailable")]
    UsernameUnavailable,

    #[error("Invalid Email Address")]
    InvalidEmail,

    #[error("Invalid Username")]
    InvalidUsername,

    #[error("Invalid Password")]
    InvalidPassword,

    #[error("Invalid Credentials")]
    InvalidCredentials,

    #[error("Insufficient Age")]
    InsufficientAge,

    #[error("Invalid Date")]
    InvalidDate,

    #[error("Invalid Message Content")]
    InvalidContent,

    #[error("Invalid Name")]
    InvalidName,

    #[error("Invalid Room Topic")]
    InvalidTopic,

    #[error("Invalid file preview")]
    InvalidPreview,

    #[error("Missing Upload-Metadata Header")]
    MissingUploadMetadataHeader,

    #[error("Missing Authorization Header")]
    MissingAuthorizationHeader,

    #[error("No Session")]
    NoSession,

    #[error("Invalid Auth Format")]
    InvalidAuthFormat,

    #[error("Header Parse Error")]
    HeaderParseError(#[from] http::header::ToStrError),

    #[error("Missing filename")]
    MissingFilename,

    #[error("Missing mime")]
    MissingMime,

    #[error("Not Found")]
    NotFound,

    #[error("Bad Request")]
    BadRequest,

    #[error("Auth Token Parse Error: {0}")]
    AuthTokenParseError(#[from] super::auth::AuthTokenFromStrError),

    #[error("Decode Error: {0}")]
    DecodeError(#[from] base64::DecodeError),

    #[error("Body Deserialization Error: {0}")]
    BodyDeserializeError(#[from] BodyDeserializeError),

    #[error("Query Parse Error: {0}")]
    QueryParseError(#[from] serde_urlencoded::de::Error),

    #[error("Checksum Mismatch")]
    ChecksumMismatch,

    #[error("Request Entity Too Large")]
    RequestEntityTooLarge,
}

impl From<db::pg::Error> for Error {
    fn from(err: db::pg::Error) -> Error {
        Error::DbError(err.into())
    }
}

impl Error {
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            Error::DbError(_)
                | Error::JoinError(_)
                | Error::SemaphoreError(_)
                | Error::HashError(_)
                | Error::JsonError(_)
                | Error::EncodingError(_)
                | Error::IOError(_)
                | Error::InternalError(_)
                | Error::InternalErrorStatic(_)
        )
    }

    pub fn http_status(&self) -> StatusCode {
        if self.is_fatal() {
            return StatusCode::INTERNAL_SERVER_ERROR;
        }

        match self {
            Error::NoSession | Error::InvalidCredentials => StatusCode::UNAUTHORIZED,
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::BadRequest => StatusCode::BAD_REQUEST,
            Error::AlreadyExists => StatusCode::CONFLICT,
            Error::MissingAuthorizationHeader
            | Error::MissingUploadMetadataHeader
            | Error::HeaderParseError(_)
            | Error::AuthTokenParseError(_)
            | Error::DecodeError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Error::ChecksumMismatch => *CHECKSUM_MISMATCH,
            Error::RequestEntityTooLarge => *REQUEST_ENTITY_TOO_LARGE,
            _ => StatusCode::BAD_REQUEST,
        }
    }

    #[rustfmt::skip]
    pub fn code(&self) -> u16 {
        match *self {
            Error::DbError(_)               => 50001,
            Error::JoinError(_)             => 50002,
            Error::SemaphoreError(_)        => 50003,
            Error::HashError(_)             => 50004,
            Error::JsonError(_)             => 50005,
            Error::EncodingError(_)         => 50006,
            Error::InternalError(_)         => 50007,
            Error::InternalErrorStatic(_)   => 50008,

            Error::AlreadyExists            => 40001,
            Error::UsernameUnavailable      => 40002,
            Error::InvalidEmail             => 40003,
            Error::InvalidUsername          => 40004,
            Error::InvalidPassword          => 40005,
            Error::InvalidCredentials       => 40006,
            Error::InsufficientAge          => 40007,
            Error::InvalidDate              => 40008,
            Error::InvalidContent           => 40009,
            Error::InvalidName              => 40010,
            Error::InvalidTopic             => 40011,
            Error::MissingUploadMetadataHeader  => 40012,
            Error::MissingAuthorizationHeader   => 40013,
            Error::NoSession                => 40014,
            Error::InvalidAuthFormat        => 40015,
            Error::HeaderParseError(_)      => 40016,
            Error::MissingFilename          => 40017,
            Error::MissingMime          => 40018,
            Error::AuthTokenParseError(_)   => 40019,
            Error::DecodeError(_)           => 40020,
            Error::BodyDeserializeError(_)  => 40021,
            Error::QueryParseError(_)       => 40022,

            // TODO: Decide on actual error codes
            _ => self.http_status().as_u16(),
        }
    }

    pub fn format(&self) -> Cow<'static, str> {
        Cow::Borrowed(match self {
            // TODO: at least say if it's a database error, for now
            Error::DbError(_) => "Database Error",
            Error::AuthTokenParseError(_) => "Auth Token Parse Error",
            Error::DecodeError(_) => "Base64 Decode Error",
            Error::IOError(_) => "IO Error",

            _ if self.is_fatal() => "Internal Server Error",
            _ => return self.to_string().into(),
        })
    }
}
