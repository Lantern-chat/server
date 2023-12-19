use std::{borrow::Cow, str::Utf8Error, string::FromUtf8Error};

use db::pool::Error as DbError;
use sdk::api::error::ApiErrorCode;
use smol_str::SmolStr;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // FATAL ERRORS
    #[error("Internal Error: {0}")]
    InternalError(String),
    #[error("Internal Error: {0}")]
    InternalErrorSmol(SmolStr),
    #[error("Internal Error: {0}")]
    InternalErrorStatic(&'static str),

    #[error("Database Error {0}")]
    DbError(DbError),
    #[error("Join Error {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Semaphore Error: {0}")]
    SemaphoreError(#[from] tokio::sync::AcquireError),

    #[error("Password Hash Error {0}")]
    HashError(#[from] argon2::Error),
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Request Error: {0}")]
    RequestError(#[from] reqwest::Error),

    // #[error("Encoding Error {0}")]
    // EventEncodingError(#[from] crate::backend::gateway::event::EventEncodingError),
    #[error("UTF8 Parse Error: {0}")]
    Utf8ParseError(#[from] FromUtf8Error),
    #[error("UTF8 Error: {0}")]
    Utf8CheckError(#[from] Utf8Error),

    #[error("Unimplemented")]
    Unimplemented,

    // NON-FATAL ERRORS
    #[error("Missing Upload-Metadata Header")]
    MissingUploadMetadataHeader,

    #[error("Missing Authorization Header")]
    MissingAuthorizationHeader,

    #[error("Missing Content Type Header")]
    MissingContentTypeHeader,

    #[error("Method Not Allowed")]
    MethodNotAllowed,

    #[error("Already Exists")]
    AlreadyExists,

    #[error("Blocked")]
    Blocked,

    #[error("Banned")]
    Banned,

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

    #[error("TOTP Required")]
    TOTPRequired,

    #[error("Insufficient Age")]
    InsufficientAge,

    #[error("Invalid Date: {0}")]
    InvalidDate(#[from] time::error::ComponentRange),

    #[error("Invalid Message Content")]
    InvalidContent,

    #[error("Invalid Name")]
    InvalidName,

    #[error("Invalid Room Topic")]
    InvalidTopic,

    #[error("Invalid file preview")]
    InvalidPreview,

    #[error("Invalid Image Format")]
    InvalidImageFormat,

    #[error("No Session")]
    NoSession,

    #[error("Invalid Auth Format")]
    InvalidAuthFormat,

    #[error("Missing filename")]
    MissingFilename,

    #[error("Missing mime")]
    MissingMime,

    #[error("Not Found")]
    NotFound,

    #[error("Bad Request")]
    BadRequest,

    #[error("Upload Error")]
    UploadError,

    #[error("Conflict/Already Exists")]
    Conflict,

    #[error("Auth Token Error: {0}")]
    AuthTokenError(#[from] schema::auth::AuthTokenError),

    #[error("Request Entity Too Large")]
    RequestEntityTooLarge,

    #[error("Temporarily Disabled")]
    TemporarilyDisabled,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Checksum Mismatch")]
    ChecksumMismatch,

    #[error("Captcha Error: {0}")]
    CaptchaError(#[from] crate::services::hcaptcha::HCaptchaError),

    #[error("Search Error: {0}")]
    SearchError(#[from] schema::search::SearchError),

    #[error("Rkyv Encoding Error")]
    RkyvEncodingError,
}

impl From<db::pg::Error> for Error {
    fn from(err: db::pg::Error) -> Error {
        Error::DbError(err.into())
    }
}

impl Error {
    #[rustfmt::skip]
    pub fn is_fatal(&self) -> bool {
        matches!(self,
            | Error::InternalError(_)
            | Error::InternalErrorSmol(_)
            | Error::InternalErrorStatic(_)
            | Error::DbError(_)
            | Error::JoinError(_)
            | Error::SemaphoreError(_)
            | Error::HashError(_)
            //| Error::EventEncodingError(_)
            | Error::IOError(_)
            | Error::RequestError(_)
            | Error::RkyvEncodingError
            | Error::Unimplemented
        )
    }

    #[inline]
    pub fn code(&self) -> u16 {
        self.to_apierror() as u16
    }

    pub fn format(&self) -> Cow<'static, str> {
        Cow::Borrowed(match self {
            _ if self.is_fatal() => "Internal Server Error",
            // TODO: at least say if it's a database error, for now
            Error::DbError(_) => "Database Error",
            Error::AuthTokenError(_) => "Auth Token Parse Error",
            Error::IOError(_) => "IO Error",
            _ => return self.to_string().into(),
        })
    }

    #[rustfmt::skip]
    pub fn to_apierror(&self) -> ApiErrorCode {
        match self {
            | Error::InternalError(_)
            | Error::InternalErrorStatic(_)
            | Error::InternalErrorSmol(_)   => ApiErrorCode::InternalError,

            Error::DbError(_)               => ApiErrorCode::DbError,
            Error::JoinError(_)             => ApiErrorCode::JoinError,
            Error::SemaphoreError(_)        => ApiErrorCode::SemaphoreError,
            Error::HashError(_)             => ApiErrorCode::HashError,
            //Error::EventEncodingError(_)    => ApiErrorCode::EventEncodingError,
            Error::RkyvEncodingError        => ApiErrorCode::RkyvEncodingError,
            | Error::Utf8ParseError(_)
            | Error::Utf8CheckError(_)      => ApiErrorCode::Utf8ParseError,
            Error::IOError(_)               => ApiErrorCode::IOError,
            Error::RequestError(_)          => ApiErrorCode::RequestError,
            Error::Unimplemented            => ApiErrorCode::Unimplemented,

            Error::AlreadyExists            => ApiErrorCode::AlreadyExists,
            Error::UsernameUnavailable      => ApiErrorCode::UsernameUnavailable,
            Error::InvalidEmail             => ApiErrorCode::InvalidEmail,
            Error::InvalidUsername          => ApiErrorCode::InvalidUsername,
            Error::InvalidPassword          => ApiErrorCode::InvalidPassword,
            Error::InvalidCredentials       => ApiErrorCode::InvalidCredentials,
            Error::InsufficientAge          => ApiErrorCode::InsufficientAge,
            Error::InvalidDate(_)           => ApiErrorCode::InvalidDate,
            Error::InvalidContent           => ApiErrorCode::InvalidContent,
            Error::InvalidName              => ApiErrorCode::InvalidName,
            Error::InvalidTopic             => ApiErrorCode::InvalidTopic,
            Error::MissingUploadMetadataHeader  => ApiErrorCode::MissingUploadMetadataHeader,
            Error::MissingAuthorizationHeader   => ApiErrorCode::MissingAuthorizationHeader,
            Error::MissingContentTypeHeader => ApiErrorCode::MissingContentTypeHeader,
            Error::NoSession                => ApiErrorCode::NoSession,
            Error::InvalidAuthFormat        => ApiErrorCode::InvalidAuthFormat,
            Error::MissingFilename          => ApiErrorCode::MissingFilename,
            Error::MissingMime              => ApiErrorCode::MissingMime,
            Error::AuthTokenError(_)        => ApiErrorCode::AuthTokenError,
            Error::UploadError              => ApiErrorCode::UploadError,
            Error::InvalidPreview           => ApiErrorCode::InvalidPreview,
            Error::InvalidImageFormat       => ApiErrorCode::InvalidImageFormat,
            Error::TOTPRequired             => ApiErrorCode::TOTPRequired,
            Error::TemporarilyDisabled      => ApiErrorCode::TemporarilyDisabled,
            Error::CaptchaError(_)          => ApiErrorCode::InvalidCaptcha,
            Error::Blocked                  => ApiErrorCode::Blocked,
            Error::Banned                   => ApiErrorCode::Banned,
            Error::SearchError(_)           => ApiErrorCode::SearchError,

            // HTTP-like error codes
            Error::BadRequest               => ApiErrorCode::BadRequest,
            Error::Unauthorized             => ApiErrorCode::Unauthorized,
            Error::NotFound                 => ApiErrorCode::NotFound,
            Error::MethodNotAllowed         => ApiErrorCode::MethodNotAllowed,
            Error::Conflict                 => ApiErrorCode::Conflict,
            Error::RequestEntityTooLarge    => ApiErrorCode::RequestEntityTooLarge,
            Error::ChecksumMismatch         => ApiErrorCode::ChecksumMismatch,
        }
    }
}

impl From<DbError> for Error {
    fn from(e: DbError) -> Self {
        if let Some(e) = e.as_db_error() {
            use db::pg::error::SqlState;

            log::warn!("DATABASE ERROR: {e}");

            // TODO: Improve this with names of specific constraints
            match *e.code() {
                SqlState::FOREIGN_KEY_VIOLATION => return Error::NotFound,
                SqlState::CHECK_VIOLATION => return Error::BadRequest,
                SqlState::UNIQUE_VIOLATION => return Error::BadRequest,
                _ => {}
            }
        }

        Error::DbError(e)
    }
}
