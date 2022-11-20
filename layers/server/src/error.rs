use std::{borrow::Cow, string::FromUtf8Error};

use db::pool::Error as DbError;
use ftl::{body::BodyDeserializeError, *};
use http::header::InvalidHeaderValue;
use sdk::{api::error::ApiErrorCode, models::UserPreferenceError};
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
    DbError(#[from] DbError),
    #[error("Join Error {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Semaphore Error: {0}")]
    SemaphoreError(#[from] tokio::sync::AcquireError),
    #[error("Parse Error {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Bincode Error: {0}")]
    BincodeError(#[from] bincode::Error),
    #[error("XML Serialize Error {0}")]
    XMLError(#[from] quick_xml::de::DeError),
    #[error("Password Hash Error {0}")]
    HashError(#[from] argon2::Error),
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Request Error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Encoding Error {0}")]
    EventEncodingError(#[from] crate::backend::gateway::event::EventEncodingError),

    #[error("UTF8 Parse Error: {0}")]
    Utf8ParseError(#[from] FromUtf8Error),

    #[error("Unimplemented")]
    Unimplemented,

    // NON-FATAL ERRORS
    #[error("Invalid Header Value: {0}")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),

    #[error("Missing Upload-Metadata Header")]
    MissingUploadMetadataHeader,

    #[error("Missing Authorization Header")]
    MissingAuthorizationHeader,

    #[error("Missing Content Type Header")]
    MissingContentTypeHeader,

    #[error("Header Parse Error")]
    HeaderParseError(#[from] http::header::ToStrError),

    #[error("Body Deserialization Error: {0}")]
    BodyDeserializeError(#[from] BodyDeserializeError),

    #[error("Query Parse Error: {0}")]
    QueryParseError(#[from] serde_urlencoded::de::Error),

    #[error("Method Not Allowed")]
    MethodNotAllowed,

    #[error("Already Exists")]
    AlreadyExists,

    #[error("Blocked")]
    Blocked,

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

    #[error(transparent)]
    InvalidPreferences(#[from] UserPreferenceError),

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

    #[error("Base-64 Decode Error: {0}")]
    Base64DecodeError(#[from] base64::DecodeError),

    #[error("Base-85 Decode Error: {0}")]
    Base85DecodeError(#[from] z85::FromZ85Error),

    #[error("Request Entity Too Large")]
    RequestEntityTooLarge,

    #[error("Temporarily Disabled")]
    TemporarilyDisabled,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Checksum Mismatch")]
    ChecksumMismatch,

    #[error("Unsupported Media Type: {0}")]
    UnsupportedMediaType(headers::ContentType),

    #[error("Mime Parse Error: {0}")]
    MimeParseError(#[from] mime::FromStrError),

    #[error("Captcha Error: {0}")]
    CaptchaError(#[from] crate::backend::services::hcaptcha::HCaptchaError),

    #[error("Websocket Error: {0}")]
    WsError(#[from] ftl::ws::WsError),
}

impl From<db::pg::Error> for Error {
    fn from(err: db::pg::Error) -> Error {
        Error::DbError(err.into())
    }
}

lazy_static::lazy_static! {
    // 460 Checksum Mismatch
    pub static ref CHECKSUM_MISMATCH: StatusCode = StatusCode::from_u16(460).unwrap();

    // 413 Request Entity Too Large
    pub static ref REQUEST_ENTITY_TOO_LARGE: StatusCode = StatusCode::from_u16(413).unwrap();
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
            | Error::JsonError(_)
            | Error::BincodeError(_)
            | Error::XMLError(_)
            | Error::EventEncodingError(_)
            | Error::IOError(_)
            | Error::RequestError(_)
            | Error::Unimplemented
        )
    }

    #[rustfmt::skip]
    pub fn http_status(&self) -> StatusCode {
        match self {
            _ if self.is_fatal() => StatusCode::INTERNAL_SERVER_ERROR,

            | Error::NoSession
            | Error::InvalidCredentials
            | Error::TOTPRequired
            | Error::Unauthorized
            | Error::CaptchaError(_) => StatusCode::UNAUTHORIZED,

            Error::TemporarilyDisabled => StatusCode::FORBIDDEN,
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::BadRequest => StatusCode::BAD_REQUEST,
            Error::AlreadyExists => StatusCode::CONFLICT,
            Error::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            Error::UnsupportedMediaType(_) | Error::MissingContentTypeHeader => StatusCode::UNSUPPORTED_MEDIA_TYPE,

            | Error::AuthTokenError(_)
            | Error::Base64DecodeError(_)
            | Error::Base85DecodeError(_)
            | Error::JsonError(_) => StatusCode::UNPROCESSABLE_ENTITY,

            Error::ChecksumMismatch => *CHECKSUM_MISMATCH,
            Error::RequestEntityTooLarge => *REQUEST_ENTITY_TOO_LARGE,
            Error::Conflict => StatusCode::CONFLICT,

            | Error::MissingAuthorizationHeader
            | Error::MissingUploadMetadataHeader
            | Error::HeaderParseError(_)
            | Error::BodyDeserializeError(_) => StatusCode::UNPROCESSABLE_ENTITY,

            Error::WsError(e) => ReplyError::status(e),

            _ => StatusCode::BAD_REQUEST,
        }
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
            Error::Base64DecodeError(_) => "Base64 Decode Error",
            Error::Base85DecodeError(_) => "Base85 Decode Error",
            Error::IOError(_) => "IO Error",
            Error::InvalidHeaderValue(_) => "Invalid Header Value",
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
            Error::JsonError(_)             => ApiErrorCode::JsonError,
            Error::BincodeError(_)          => ApiErrorCode::BincodeError,
            Error::EventEncodingError(_)    => ApiErrorCode::EventEncodingError,
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
            Error::InvalidDate(_)           => ApiErrorCode::InvalidDate,
            Error::InvalidContent           => ApiErrorCode::InvalidContent,
            Error::InvalidName              => ApiErrorCode::InvalidName,
            Error::InvalidTopic             => ApiErrorCode::InvalidTopic,
            Error::MissingUploadMetadataHeader  => ApiErrorCode::MissingUploadMetadataHeader,
            Error::MissingAuthorizationHeader   => ApiErrorCode::MissingAuthorizationHeader,
            Error::MissingContentTypeHeader => ApiErrorCode::MissingContentTypeHeader,
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
            Error::CaptchaError(_)          => ApiErrorCode::InvalidCaptcha,
            Error::Base85DecodeError(_)     => ApiErrorCode::Base85DecodeError,
            Error::WsError(_)               => ApiErrorCode::WebsocketError,
            Error::Blocked                  => ApiErrorCode::Blocked,

            // HTTP-like error codes
            Error::BadRequest               => ApiErrorCode::BadRequest,
            Error::Unauthorized             => ApiErrorCode::Unauthorized,
            Error::NotFound                 => ApiErrorCode::NotFound,
            Error::MethodNotAllowed         => ApiErrorCode::MethodNotAllowed,
            Error::Conflict                 => ApiErrorCode::Conflict,
            Error::RequestEntityTooLarge    => ApiErrorCode::RequestEntityTooLarge,
            Error::UnsupportedMediaType(_)  => ApiErrorCode::UnsupportedMediaType,
            Error::ChecksumMismatch         => ApiErrorCode::ChecksumMismatch,
        }
    }

    fn into_json(self) -> reply::Json {
        if self.is_fatal() {
            log::error!("Fatal error: {self}");
        } else if cfg!(debug_assertions) {
            log::warn!("Non-fatal error: {self}");
        }

        #[derive(Serialize)]
        pub struct ApiError {
            pub code: ApiErrorCode,
            pub message: Cow<'static, str>,
        }

        reply::json(&ApiError {
            code: self.to_apierror(),
            message: self.format(),
        })
    }

    #[allow(non_upper_case_globals)]
    fn into_cached_json(self) -> reply::Json {
        use reply::Json;

        macro_rules! impl_cached {
            ($($name:ident),*) => {{
                lazy_static::lazy_static! {$(
                    static ref $name: Json = Error::$name.into_json();
                )*}

                match self {
                    $(Error::$name => $name.clone(),)*
                    _ => self.into_json(),
                }
            }}
        }

        impl_cached! {
            NotFound,
            BadRequest,
            NoSession,
            InvalidCredentials,
            AlreadyExists,
            Blocked
        }
    }
}

impl ftl::Reply for Error {
    fn into_response(self) -> Response {
        self.into_cached_json().into_response()
    }
}

impl ftl::ReplyError for Error {
    fn status(&self) -> StatusCode {
        self.http_status()
    }
}
