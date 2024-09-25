use std::{borrow::Cow, str::Utf8Error, string::FromUtf8Error, time::Duration};

use db::Error as DbError;
use ftl::ws::WsError;
//use ftl::{body::BodyError, ws::WsRejection};
use http::{header::InvalidHeaderValue, StatusCode};
use sdk::{
    api::error::{ApiError, ApiErrorCode},
    driver::Encoding,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("API Error")]
    ApiError(ApiError),

    #[error("Not Found")]
    NotFound,
    /// Signals that an invalid path was requested
    #[error("Not Found")]
    NotFoundSignaling,
    /// Signals that a high penalty should be applied
    #[error("Not Found")]
    NotFoundHighPenalty,

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
    // #[error("XML Error {0}")]
    // XMLError(#[from] quick_xml::Error),
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

    //#[error("Body Deserialization Error: {0}")]
    //BodyDeserializeError(#[from] BodyDeserializeError),
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

    #[error("Unsupported Media Type")]
    UnsupportedMediaTypeGeneric,

    #[error("Mime Parse Error: {0}")]
    MimeParseError(#[from] mime::FromStrError),
    #[error("Websocket Error: {0}")]
    WsError(#[from] ftl::ws::WsError),
    // #[error("Search Error: {0}")]
    // SearchError(#[from] schema::search::SearchError),
}

impl From<db::pg::Error> for Error {
    fn from(err: db::pg::Error) -> Error {
        Error::DbError(err.into())
    }
}

use std::sync::LazyLock;

// 460 Checksum Mismatch
pub static CHECKSUM_MISMATCH: LazyLock<StatusCode> = LazyLock::new(|| StatusCode::from_u16(460).unwrap());
// 413 Request Entity Too Large
pub static REQUEST_ENTITY_TOO_LARGE: LazyLock<StatusCode> = LazyLock::new(|| StatusCode::from_u16(413).unwrap());

impl Error {
    /// Rate-limiting penalty in milliseconds
    pub fn penalty(&self) -> Duration {
        // TODO: Make this configurable or find better values
        Duration::from_millis(match self {
            Error::NotFoundSignaling => 100,
            Error::NotFoundHighPenalty => 500,
            Error::BadRequest => 200,
            Error::Unauthorized => 300,
            Error::MethodNotAllowed => 200,
            _ => 0,
        })
    }

    /// Returns whether the error is fatal and should be logged as an error
    #[rustfmt::skip]
    pub fn is_fatal(&self) -> bool {
        match self {
            | Error::DbError(_)
            | Error::JoinError(_)
            | Error::SemaphoreError(_)
            | Error::JsonError(_)
            | Error::CborDecodingError(_)
            | Error::CborEncodingError(_)
            //| Error::XMLError(_)
            | Error::IOError(_)
            | Error::RequestError(_) => true,

            Error::ApiError(e) if e.code == ApiErrorCode::InternalError => true,

            _ => false,
        }
    }
}

impl From<Error> for sdk::api::error::ApiError {
    #[rustfmt::skip]
    fn from(value: Error) -> Self {
        let message = 'msg: {
            Cow::Borrowed(match value {
                Error::ApiError(err) => return err,
                Error::IOError(err) => return err.into(),

                // TODO: at least say if it's a database error, for now
                Error::DbError(_)                   => "Database Error",
                Error::AuthTokenError(_)            => "Auth Token Parse Error",
                Error::Base64DecodeError(_)         => "Base64 Decode Error",
                Error::Base85DecodeError(_)         => "Base85 Decode Error",
                Error::InvalidHeaderValue(_)        => "Invalid Header Value",
                Error::NotFound                     => "Not Found",
                Error::NotFoundSignaling            => "Not Found",
                Error::BadRequest                   => "Bad Request",
                Error::MethodNotAllowed             => "Method Not Allowed",
                Error::RequestEntityTooLarge        => "Request Entity Too Large",
                Error::MissingAuthorizationHeader   => "Missing Authorization Header",
                Error::Unauthorized                 => "Unauthorized",
                Error::NoSession                    => "No Session",
                Error::Unimplemented                => "Unimplemented",

                _ if value.is_fatal() => {
                    return Self {
                        code: ApiErrorCode::InternalError,
                        message: Cow::Borrowed("Internal Server Error"),
                    };
                }

                _ => break 'msg value.to_string().into(),
            })
        };

        let code = match value {
            Error::NotFound                     => ApiErrorCode::NotFound,
            Error::NotFoundSignaling            => ApiErrorCode::NotFound,
            Error::NotFoundHighPenalty          => ApiErrorCode::NotFound,
            Error::BadRequest                   => ApiErrorCode::BadRequest,
            Error::MethodNotAllowed             => ApiErrorCode::MethodNotAllowed,
            Error::RequestEntityTooLarge        => ApiErrorCode::RequestEntityTooLarge,
            Error::MissingAuthorizationHeader   => ApiErrorCode::MissingAuthorizationHeader,
            Error::Unauthorized                 => ApiErrorCode::Unauthorized,
            Error::NoSession                    => ApiErrorCode::NoSession,
            Error::Unimplemented                => ApiErrorCode::Unimplemented,

            Error::IOError(_)               => ApiErrorCode::IOError,
            Error::DbError(_)               => ApiErrorCode::DbError,
            Error::JoinError(_)             => ApiErrorCode::JoinError,
            Error::SemaphoreError(_)        => ApiErrorCode::SemaphoreError,
            //Error::XMLError(_)              => ApiErrorCode::XMLError,
            Error::RequestError(_)          => ApiErrorCode::RequestError,
            Error::Utf8ParseError(_)        => ApiErrorCode::Utf8ParseError,
            Error::Utf8CheckError(_)        => ApiErrorCode::Utf8ParseError, // revisit
            Error::InvalidHeaderValue(_)    => ApiErrorCode::InvalidHeaderValue,
            Error::QueryParseError(_)       => ApiErrorCode::QueryParseError,
            Error::MimeParseError(_)        => ApiErrorCode::MimeParseError,
            //Error::SearchError(_)           => ApiErrorCode::SearchError,

            Error::UnsupportedMediaType(_)  => ApiErrorCode::UnsupportedMediaType,
            Error::UnsupportedMediaTypeGeneric => ApiErrorCode::UnsupportedMediaType,

            Error::AuthTokenError(_)        => ApiErrorCode::AuthTokenError,
            Error::Base64DecodeError(_)     => ApiErrorCode::Base64DecodeError,
            Error::Base85DecodeError(_)     => ApiErrorCode::Base85DecodeError,
            Error::CborDecodingError(_) |
            Error::CborEncodingError(_)     => ApiErrorCode::CborError,
            Error::JsonError(_)             => ApiErrorCode::JsonError,

            Error::HeaderParseError(_)      => ApiErrorCode::HeaderParseError,
            //Error::BodyDeserializeError(_)  => ApiErrorCode::BodyDeserializeError,

            Error::WsError(
                | WsError::MethodNotConnect
                | WsError::MethodNotGet
            ) => ApiErrorCode::MethodNotAllowed,

            Error::WsError(
                | WsError::IncorrectWebSocketVersion
                | WsError::InvalidProtocolPsuedoHeader
                | WsError::MissingWebSocketKey
            ) => ApiErrorCode::BadRequest,

            Error::WsError(_)               => ApiErrorCode::WebsocketError,

            | Error::InternalError(_)
            | Error::InternalErrorStatic(_)
            | Error::InternalErrorSmol(_)   => ApiErrorCode::InternalError,

            Error::ApiError(_) => unreachable!(),
        };

        Self { code, message }
    }
}

impl ftl::IntoResponse for Error {
    // TODO: Choose format based on the deferred response system
    fn into_response(self) -> ftl::Response {
        let err = ApiError::from(self);
        let status = err.code.http_status();
        ftl::IntoResponse::into_response((ftl::body::deferred::Deferred::new(err), status))
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

    // fn into_json(self) -> WithStatus<reply::Json> {
    //     let err = ApiError::from(self);
    //     reply::json(&err).with_status(err.code.http_status())
    // }

    // fn into_cbor(self) -> WithStatus<reply::cbor::Cbor> {
    //     let err = ApiError::from(self);
    //     reply::cbor::cbor(&err).with_status(err.code.http_status())
    // }

    // #[allow(non_upper_case_globals)]
    // fn into_cached_json(self) -> WithStatus<reply::Json> {
    //     use reply::Json;

    //     macro_rules! impl_cached {
    //         ($($name:ident),*) => {{
    //             $(static $name: LazyLock<WithStatus<Json>> = LazyLock::new(|| Error::$name.into_json());)*

    //             match self {
    //                 $(Self::$name => $name.clone(),)*
    //                 _ => self.into_json(),
    //             }
    //         }}
    //     }

    //     impl_cached! {
    //         NotFound,
    //         NotFoundSignaling,
    //         NotFoundHighPenalty,
    //         BadRequest,
    //         MethodNotAllowed,
    //         Unimplemented,
    //         RequestEntityTooLarge,
    //         MissingAuthorizationHeader,
    //         Unauthorized,
    //         NoSession
    //     }
    // }

    // pub(crate) fn into_encoding(self, encoding: Encoding) -> Response {
    //     self.log();

    //     match encoding {
    //         Encoding::JSON => self.into_cached_json().into_response(),
    //         Encoding::CBOR => self.into_cbor().into_response(),
    //     }
    // }
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

impl From<ApiError> for Error {
    fn from(e: ApiError) -> Self {
        // These are more for faster formatting than anything else
        match e.code {
            ApiErrorCode::NotFound => Error::NotFound,
            ApiErrorCode::BadRequest => Error::BadRequest,
            ApiErrorCode::MethodNotAllowed => Error::MethodNotAllowed,
            ApiErrorCode::Unimplemented => Error::Unimplemented,
            ApiErrorCode::RequestEntityTooLarge => Error::RequestEntityTooLarge,
            ApiErrorCode::MissingAuthorizationHeader => Error::MissingAuthorizationHeader,
            ApiErrorCode::Unauthorized => Error::Unauthorized,
            ApiErrorCode::NoSession => Error::NoSession,
            _ => Error::ApiError(e),
        }
    }
}

impl From<core::convert::Infallible> for Error {
    #[inline(always)]
    fn from(e: core::convert::Infallible) -> Self {
        match e {}
    }
}

impl From<ftl::Error> for Error {
    fn from(value: ftl::Error) -> Self {
        match value {
            ftl::Error::Unauthorized => Error::MissingAuthorizationHeader, // the only way this occurs is if the auth header is missing

            ftl::Error::HyperError(_) => Error::InternalErrorStatic("HTTP Transport Error"),
            ftl::Error::BodyError(body_error) => match body_error {
                ftl::body::BodyError::HyperError(error) => Error::InternalErrorStatic("Error Reading Body"),
                ftl::body::BodyError::Io(error) => Error::IOError(error),
                ftl::body::BodyError::StreamAborted => Error::BadRequest,
                ftl::body::BodyError::LengthLimitError(_) => Error::RequestEntityTooLarge,
                ftl::body::BodyError::Generic(error) => unimplemented!(),
                ftl::body::BodyError::DeferredNotConverted => {
                    Error::InternalErrorStatic("Deferred Body Not Converted")
                }
                ftl::body::BodyError::ArbitraryBodyPolled => Error::InternalErrorStatic("Arbitrary Body Polled"),
            },
            ftl::Error::Utf8Error(_) => todo!(),
            ftl::Error::StreamAborted => todo!(),
            ftl::Error::BadRequest => Error::BadRequest,
            ftl::Error::MissingHeader(h) => match h {
                "Authorization" => Error::MissingAuthorizationHeader,
                _ => Error::BadRequest,
            },
            ftl::Error::InvalidHeader(_, error) => Error::BadRequest,
            ftl::Error::NotFound => Error::NotFound,
            ftl::Error::MethodNotAllowed => Error::MethodNotAllowed,
            ftl::Error::UnsupportedMediaType => Error::UnsupportedMediaTypeGeneric,
            ftl::Error::PayloadTooLarge => Error::RequestEntityTooLarge,
            ftl::Error::MissingExtension => Error::InternalErrorStatic("Missing Extension in API Service"),
            ftl::Error::MissingQuery => Error::BadRequest,
            ftl::Error::MissingMatchedPath => Error::InternalErrorStatic("Missing Matched Path in API Service"),
            // TODO: Improve these error messages
            ftl::Error::Form(_error) => Error::BadRequest,
            ftl::Error::Cbor(_error) => Error::BadRequest,
            ftl::Error::Json(_error) => Error::BadRequest,

            ftl::Error::Path(_) => Error::BadRequest,
            ftl::Error::Scheme(_) => Error::BadRequest,
            ftl::Error::Authority(_) => Error::BadRequest,
            ftl::Error::WebsocketError(ws_error) => ws_error.into(),

            // currently unused
            ftl::Error::Custom(_error) => todo!(),
        }
    }
}
