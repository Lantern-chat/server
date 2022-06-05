use std::{borrow::Cow, string::FromUtf8Error};

use db::pool::Error as DbError;
use sdk::models::UserPreferenceError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // FATAL ERRORS
    #[error("Database Error {0}")]
    DbError(#[from] DbError),
    #[error("Join Error {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Semaphore Error: {0}")]
    SemaphoreError(#[from] tokio::sync::AcquireError),
    #[error("Parse Error {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Password Hash Error {0}")]
    HashError(#[from] argon2::Error),
    #[error("IO Error: {0}")]
    IOError(std::io::Error),
    #[error("Internal Error: {0}")]
    InternalError(String),
    #[error("Internal Error: {0}")]
    InternalErrorStatic(&'static str),
    #[error("Request Error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Encoding Error {0}")]
    EventEncodingError(#[from] crate::gateway::event::EventEncodingError),

    #[error("Unimplemented")]
    Unimplemented,

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

    #[error("{0}")]
    InvalidPreferences(UserPreferenceError),

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

    #[error("Upload Conflict")]
    Conflict,

    #[error("Auth Token Error: {0}")]
    AuthTokenError(#[from] schema::auth::AuthTokenError),

    #[error("Base-64 Decode Error: {0}")]
    Base64DecodeError(#[from] base64::DecodeError),

    #[error("Base-85 Decode Error: {0}")]
    Base85DecodeError(#[from] blurhash::base85::FromZ85Error),

    #[error("Request Entity Too Large")]
    RequestEntityTooLarge,

    #[error("Temporarily Disabled")]
    TemporarilyDisabled,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Mime Parse Error: {0}")]
    MimeParseError(#[from] mime::FromStrError),

    #[error("Captcha Error: {0}")]
    CaptchaError(#[from] crate::services::hcaptcha::HCaptchaError),
}

impl From<db::pg::Error> for Error {
    fn from(err: db::pg::Error) -> Error {
        Error::DbError(err.into())
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        if err.kind() == std::io::ErrorKind::NotFound {
            Error::NotFound
        } else {
            Error::IOError(err)
        }
    }
}
