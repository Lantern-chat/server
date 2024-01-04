use std::borrow::Cow;

pub use sdk::api::error::{ApiError, ApiErrorCode};

#[derive(Debug, thiserror::Error)]
pub enum Error {
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
}

impl From<Error> for ApiError {
    fn from(value: Error) -> Self {
        let message = 'msg: {
            Cow::Borrowed(match value {
                _ if value.is_fatal() => "Internal Server Error",
                _ => break 'msg value.to_string().into(),
            })
        };

        #[rustfmt::skip]
        let code = match value {
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

impl Error {
    #[rustfmt::skip]
    pub fn is_fatal(&self) -> bool {
        matches!(self,
            | Error::RkyvEncodingError
            | Error::Unimplemented
        )
    }
}
