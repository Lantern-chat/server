use std::borrow::Cow;

use db::Error as DbError;
use sdk::api::error::ApiErrorCode;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Internal Error: {0}")]
    InternalError(String),
    #[error("Internal Error: {0}")]
    InternalErrorSmol(smol_str::SmolStr),
    #[error("Internal Error: {0}")]
    InternalErrorStatic(&'static str),

    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),

    // FATAL ERRORS
    #[error("Database Error {0}")]
    DbError(DbError),
    #[error("Join Error {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Semaphore Error: {0}")]
    SemaphoreError(#[from] tokio::sync::AcquireError),

    #[error("UTF8 Parse Error: {0}")]
    Utf8ParseError(#[from] std::string::FromUtf8Error),

    #[error("UTF8 Error: {0}")]
    Utf8CheckError(#[from] std::str::Utf8Error),

    #[cfg(feature = "rust-argon2")]
    #[error("Password Hash Error {0}")]
    RustArgon2HashError(#[from] rust_argon2::Error),

    #[cfg(feature = "rustcrypto-argon2")]
    #[error("Password Hash Error: {0}")]
    RustCryptoArgon2HashError(#[from] rustcrypto_argon2::Error),
    #[cfg(feature = "rustcrypto-argon2")]
    #[error("Password Hash Error: {0}")]
    RustCryptoArgon2PasswordHashError(#[from] rustcrypto_argon2::password_hash::Error),

    #[error("Request Error: {0}")]
    RequestError(#[from] reqwest::Error),

    // #[error("Encoding Error {0}")]
    // EventEncodingError(#[from] crate::backend::gateway::event::EventEncodingError),

    // NON-FATAL ERRORS
    #[error("Captcha Error: {0}")]
    CaptchaError(#[from] crate::services::hcaptcha::HCaptchaError),

    #[error("Search Error: {0}")]
    SearchError(#[from] schema::search::SearchError),

    #[error("Auth Token Error: {0}")]
    AuthTokenError(#[from] schema::auth::AuthTokenError),

    #[error("Unimplemented")]
    Unimplemented,

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

    #[error("Request Entity Too Large")]
    RequestEntityTooLarge,

    #[error("Temporarily Disabled")]
    TemporarilyDisabled,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Checksum Mismatch")]
    ChecksumMismatch,

    #[error("Rkyv Encoding Error")]
    RkyvEncodingError,

    #[error("Invalid RPC Endpoint")]
    InvalidRpcEndpoint,
}

impl From<db::pg::Error> for Error {
    fn from(err: db::pg::Error) -> Error {
        Error::DbError(err.into())
    }
}

impl Error {
    #[rustfmt::skip]
    pub fn is_fatal(&self) -> bool {
        match self {
            | Error::Unimplemented
            | Error::InternalError(_)
            | Error::InternalErrorSmol(_)
            | Error::InternalErrorStatic(_)
            | Error::RkyvEncodingError
            | Error::DbError(_)
            | Error::JoinError(_)
            | Error::SemaphoreError(_)
            | Error::RequestError(_) => true,

            #[cfg(feature = "rust-argon2")]
            | Error::RustArgon2HashError(_) => true,

            #[cfg(feature = "rustcrypto-argon2")]
            | Error::RustCryptoArgon2HashError(_)
            | Error::RustCryptoArgon2PasswordHashError(_) => true,

            _ => false,
        }
    }
}

impl From<Error> for sdk::api::error::ApiError {
    fn from(value: Error) -> Self {
        let message = 'msg: {
            Cow::Borrowed(match value {
                _ if value.is_fatal() => "Internal Server Error",
                // TODO: at least say if it's a database error, for now
                Error::DbError(_) => "Database Error",
                Error::AuthTokenError(_) => "Auth Token Parse Error",
                Error::IOError(_) => "IO Error",
                Error::Unimplemented => "Unimplemented",
                Error::MissingUploadMetadataHeader => "Missing Upload-Metadata Header",
                Error::MissingAuthorizationHeader => "Missing Authorization Header",
                Error::MissingContentTypeHeader => "Missing Content Type Header",
                Error::MethodNotAllowed => "Method Not Allowed",
                Error::AlreadyExists => "Already Exists",
                Error::Blocked => "Blocked",
                Error::Banned => "Banned",
                Error::UsernameUnavailable => "Username Unavailable",
                Error::InvalidEmail => "Invalid Email Address",
                Error::InvalidUsername => "Invalid Username",
                Error::InvalidPassword => "Invalid Password",
                Error::InvalidCredentials => "Invalid Credentials",
                Error::TOTPRequired => "TOTP Required",
                Error::InsufficientAge => "Insufficient Age",
                Error::InvalidContent => "Invalid Message Content",
                Error::InvalidName => "Invalid Name",
                Error::InvalidTopic => "Invalid Room Topic",
                Error::InvalidPreview => "Invalid file preview",
                Error::InvalidImageFormat => "Invalid Image Format",
                Error::NoSession => "No Session",
                Error::InvalidAuthFormat => "Invalid Auth Format",
                Error::MissingFilename => "Missing filename",
                Error::MissingMime => "Missing mime",
                Error::NotFound => "Not Found",
                Error::BadRequest => "Bad Request",
                Error::UploadError => "Upload Error",
                Error::Conflict => "Conflict/Already Exists",
                Error::RequestEntityTooLarge => "Request Entity Too Large",
                Error::TemporarilyDisabled => "Temporarily Disabled",
                Error::Unauthorized => "Unauthorized",
                Error::ChecksumMismatch => "Checksum Mismatch",
                Error::RkyvEncodingError => "Rkyv Encoding Error",
                Error::InvalidRpcEndpoint => "Invalid RPC Endpoint",

                _ => break 'msg value.to_string().into(),
            })
        };

        #[rustfmt::skip]
        let code = match value {
            | Error::InternalError(_)
            | Error::InternalErrorStatic(_)
            | Error::InternalErrorSmol(_)   => ApiErrorCode::InternalError,

            Error::DbError(_)               => ApiErrorCode::DbError,
            Error::JoinError(_)             => ApiErrorCode::JoinError,
            Error::SemaphoreError(_)        => ApiErrorCode::SemaphoreError,

            #[cfg(feature = "rust-argon2")]
            Error::RustArgon2HashError(_)   => ApiErrorCode::HashError,

            #[cfg(feature = "rustcrypto-argon2")]
            Error::RustCryptoArgon2HashError(_) => ApiErrorCode::HashError,
            #[cfg(feature = "rustcrypto-argon2")]
            Error::RustCryptoArgon2PasswordHashError(_) => ApiErrorCode::HashError,
            Error::RequestError(_)          => ApiErrorCode::RequestError,
            Error::IOError(_)               => ApiErrorCode::IOError,

            | Error::Utf8ParseError(_)
            | Error::Utf8CheckError(_)      => ApiErrorCode::Utf8ParseError,

            Error::AuthTokenError(_)        => ApiErrorCode::AuthTokenError,
            Error::CaptchaError(_)          => ApiErrorCode::InvalidCaptcha,
            Error::SearchError(_)           => ApiErrorCode::SearchError,

            Error::RkyvEncodingError            => ApiErrorCode::RkyvEncodingError,
            Error::Unimplemented                => ApiErrorCode::Unimplemented,

            Error::AlreadyExists                => ApiErrorCode::AlreadyExists,
            Error::UsernameUnavailable          => ApiErrorCode::UsernameUnavailable,
            Error::InvalidEmail                 => ApiErrorCode::InvalidEmail,
            Error::InvalidUsername              => ApiErrorCode::InvalidUsername,
            Error::InvalidPassword              => ApiErrorCode::InvalidPassword,
            Error::InvalidCredentials           => ApiErrorCode::InvalidCredentials,
            Error::InsufficientAge              => ApiErrorCode::InsufficientAge,
            Error::InvalidContent               => ApiErrorCode::InvalidContent,
            Error::InvalidName                  => ApiErrorCode::InvalidName,
            Error::InvalidTopic                 => ApiErrorCode::InvalidTopic,
            Error::MissingUploadMetadataHeader  => ApiErrorCode::MissingUploadMetadataHeader,
            Error::MissingAuthorizationHeader   => ApiErrorCode::MissingAuthorizationHeader,
            Error::MissingContentTypeHeader     => ApiErrorCode::MissingContentTypeHeader,
            Error::NoSession                    => ApiErrorCode::NoSession,
            Error::InvalidAuthFormat            => ApiErrorCode::InvalidAuthFormat,
            Error::MissingFilename              => ApiErrorCode::MissingFilename,
            Error::MissingMime                  => ApiErrorCode::MissingMime,
            Error::UploadError                  => ApiErrorCode::UploadError,
            Error::InvalidPreview               => ApiErrorCode::InvalidPreview,
            Error::InvalidImageFormat           => ApiErrorCode::InvalidImageFormat,
            Error::TOTPRequired                 => ApiErrorCode::TOTPRequired,
            Error::TemporarilyDisabled          => ApiErrorCode::TemporarilyDisabled,
            Error::Blocked                      => ApiErrorCode::Blocked,
            Error::Banned                       => ApiErrorCode::Banned,
            Error::InvalidRpcEndpoint           => ApiErrorCode::NotFound,

            // HTTP-like error codes
            Error::BadRequest               => ApiErrorCode::BadRequest,
            Error::Unauthorized             => ApiErrorCode::Unauthorized,
            Error::NotFound                 => ApiErrorCode::NotFound,
            Error::MethodNotAllowed         => ApiErrorCode::MethodNotAllowed,
            Error::Conflict                 => ApiErrorCode::Conflict,
            Error::RequestEntityTooLarge    => ApiErrorCode::RequestEntityTooLarge,
            Error::ChecksumMismatch         => ApiErrorCode::ChecksumMismatch,
        };

        Self { code, message }
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
