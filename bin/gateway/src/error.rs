use std::{borrow::Cow, str::Utf8Error, string::FromUtf8Error};

use crate::prelude::*;

use db::pool::Error as DbError;
use ftl::{body::BodyDeserializeError, reply::WithStatus, ws::WsError, *};
use http::header::InvalidHeaderValue;
use sdk::{
    api::error::{ApiError, ApiErrorCode},
    driver::Encoding,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // Common Errors
    #[error("Not Found")]
    NotFound,
    #[error("Bad Request")]
    BadRequest,
    #[error("Method Not Allowed")]
    MethodNotAllowed,
    #[error("Unimplemented")]
    Unimplemented,
    #[error("Request Entity Too Large")]
    RequestEntityTooLarge,
    #[error("Missing Authorization Header")]
    MissingAuthorizationHeader,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("No Session")]
    NoSession,

    // FATAL ERRORS
    #[error("Internal Error: {0}")]
    InternalError(String),
    #[error("Internal Error: {0}")]
    InternalErrorSmol(smol_str::SmolStr),
    #[error("Internal Error: {0}")]
    InternalErrorStatic(&'static str),

    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("Database Error {0}")]
    DbError(DbError),
    #[error("Join Error {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Semaphore Error: {0}")]
    SemaphoreError(#[from] tokio::sync::AcquireError),
    #[error("Parse Error {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Cbor Encoding Error: {0}")]
    CborEncodingError(#[from] ciborium::ser::Error<std::io::Error>),
    #[error("Cbor Decoding Error: {0}")]
    CborDecodingError(#[from] ciborium::de::Error<std::io::Error>),
    #[error("XML Error {0}")]
    XMLError(#[from] quick_xml::Error),
    #[error("Request Error: {0}")]
    RequestError(#[from] reqwest::Error),

    //#[error("Encoding Error {0}")]
    //EventEncodingError(#[from] crate::backend::gateway::event::EventEncodingError),
    #[error("UTF8 Parse Error: {0}")]
    Utf8ParseError(#[from] FromUtf8Error),
    #[error("UTF8 Error: {0}")]
    Utf8CheckError(#[from] Utf8Error),

    // NON-FATAL ERRORS
    #[error("Invalid Header Value: {0}")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),

    #[error("Header Parse Error")]
    HeaderParseError(#[from] http::header::ToStrError),

    #[error("Body Deserialization Error: {0}")]
    BodyDeserializeError(#[from] BodyDeserializeError),

    #[error("Query Parse Error: {0}")]
    QueryParseError(#[from] serde_urlencoded::de::Error),

    #[error("Auth Token Error: {0}")]
    AuthTokenError(#[from] schema::auth::AuthTokenError),

    #[error("Base-64 Decode Error: {0}")]
    Base64DecodeError(#[from] base64::DecodeError),

    #[error("Base-85 Decode Error: {0}")]
    Base85DecodeError(#[from] z85::FromZ85Error),

    #[error("Unsupported Media Type: {0}")]
    UnsupportedMediaType(headers::ContentType),

    #[error("Mime Parse Error: {0}")]
    MimeParseError(#[from] mime::FromStrError),

    #[error("Websocket Error: {0}")]
    WsError(#[from] ftl::ws::WsError),

    #[error("Search Error: {0}")]
    SearchError(#[from] schema::search::SearchError),
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
            | Error::Unimplemented
            | Error::DbError(_)
            | Error::JoinError(_)
            | Error::SemaphoreError(_)
            | Error::JsonError(_)
            | Error::CborDecodingError(_)
            | Error::CborEncodingError(_)

            | Error::XMLError(_)
            | Error::IOError(_)
            | Error::RequestError(_)
        )
    }
}

impl From<Error> for sdk::api::error::ApiError {
    fn from(value: Error) -> Self {
        let message = 'msg: {
            Cow::Borrowed(match value {
                _ if value.is_fatal() => {
                    return Self {
                        code: ApiErrorCode::InternalError,
                        message: "Internal Server Error".into(),
                    }
                }

                // TODO: at least say if it's a database error, for now
                Error::DbError(_) => "Database Error",
                Error::AuthTokenError(_) => "Auth Token Parse Error",
                Error::Base64DecodeError(_) => "Base64 Decode Error",
                Error::Base85DecodeError(_) => "Base85 Decode Error",
                Error::IOError(_) => "IO Error",
                Error::InvalidHeaderValue(_) => "Invalid Header Value",
                Error::NotFound => "Not Found",
                Error::BadRequest => "Bad Request",
                Error::MethodNotAllowed => "Method Not Allowed",
                Error::RequestEntityTooLarge => "Request Entity Too Large",
                Error::MissingAuthorizationHeader => "Missing Authorization Header",
                Error::Unauthorized => "Unauthorized",

                _ => break 'msg value.to_string().into(),
            })
        };

        #[rustfmt::skip]
        let code = match value {
            Error::NotFound                     => ApiErrorCode::NotFound,
            Error::BadRequest                   => ApiErrorCode::BadRequest,
            Error::MethodNotAllowed             => ApiErrorCode::MethodNotAllowed,
            Error::RequestEntityTooLarge        => ApiErrorCode::RequestEntityTooLarge,
            Error::MissingAuthorizationHeader   => ApiErrorCode::MissingAuthorizationHeader,
            Error::Unauthorized                 => ApiErrorCode::Unauthorized,
            Error::NoSession                    => ApiErrorCode::NoSession,

            Error::IOError(_)               => ApiErrorCode::IOError,
            Error::DbError(_)               => ApiErrorCode::DbError,
            Error::JoinError(_)             => ApiErrorCode::JoinError,
            Error::SemaphoreError(_)        => ApiErrorCode::SemaphoreError,
            Error::XMLError(_)              => ApiErrorCode::XMLError,
            Error::RequestError(_)          => ApiErrorCode::RequestError,
            Error::Utf8ParseError(_)        => ApiErrorCode::Utf8ParseError,
            Error::Utf8CheckError(_)        => ApiErrorCode::Utf8ParseError, // revisit
            Error::InvalidHeaderValue(_)    => ApiErrorCode::InvalidHeaderValue,
            Error::QueryParseError(_)       => ApiErrorCode::QueryParseError,
            Error::MimeParseError(_)        => ApiErrorCode::MimeParseError,
            Error::SearchError(_)           => ApiErrorCode::SearchError,

            Error::UnsupportedMediaType(_)  => ApiErrorCode::UnsupportedMediaType,

            Error::AuthTokenError(_)        => ApiErrorCode::AuthTokenError,
            Error::Base64DecodeError(_)     => ApiErrorCode::Base64DecodeError,
            Error::Base85DecodeError(_)     => ApiErrorCode::Base85DecodeError,
            Error::CborDecodingError(_) |
            Error::CborEncodingError(_)     => ApiErrorCode::CborError,
            Error::JsonError(_)             => ApiErrorCode::JsonError,

            Error::HeaderParseError(_)      => ApiErrorCode::HeaderParseError,
            Error::BodyDeserializeError(_)  => ApiErrorCode::BodyDeserializeError,

            Error::WsError(WsError::MethodNotAllowed)   => ApiErrorCode::MethodNotAllowed,
            Error::WsError(_)                           => ApiErrorCode::WebsocketError,

            | Error::Unimplemented
            | Error::InternalError(_)
            | Error::InternalErrorStatic(_)
            | Error::InternalErrorSmol(_) => unreachable!(),
        };

        Self { code, message }
    }
}

impl Error {
    fn log(&self) {
        if self.is_fatal() {
            log::error!("Fatal error: {self}");
        } else if cfg!(debug_assertions) {
            log::warn!("Non-fatal error: {self}");
        }
    }

    fn into_json(self) -> WithStatus<reply::Json> {
        let err = ApiError::from(self);
        reply::json(&err).with_status(err.code.http_status())
    }

    fn into_cbor(self) -> WithStatus<reply::cbor::Cbor> {
        let err = ApiError::from(self);
        reply::cbor::cbor(&err).with_status(err.code.http_status())
    }

    #[allow(non_upper_case_globals)]
    fn into_cached_json(self) -> WithStatus<reply::Json> {
        use reply::Json;

        macro_rules! impl_cached {
            ($($name:ident),*) => {{
                lazy_static::lazy_static! {$(
                    static ref $name: WithStatus<Json> = Error::$name.into_json();
                )*}

                match self {
                    $(Self::$name => $name.clone(),)*
                    _ => self.into_json(),
                }
            }}
        }

        impl_cached! {
            NotFound,
            BadRequest,
            MethodNotAllowed,
            Unimplemented,
            RequestEntityTooLarge,
            MissingAuthorizationHeader,
            Unauthorized,
            NoSession
        }
    }

    pub(crate) fn into_encoding(self, encoding: Encoding) -> Response {
        self.log();

        match encoding {
            Encoding::JSON => self.into_cached_json().into_response(),
            Encoding::CBOR => self.into_cbor().into_response(),
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
