use std::borrow::Cow;

use rpc::error::Error as CommonError;

use db::pool::Error as DbError;
use sdk::api::error::ApiErrorCode;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Common(#[from] CommonError),

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

    #[error("Password Hash Error {0}")]
    HashError(#[from] argon2::Error),

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
            Error::Common(err) => err.is_fatal(),
            _ => matches!(self,
                | Error::InternalError(_)
                | Error::InternalErrorSmol(_)
                | Error::InternalErrorStatic(_)

                | Error::DbError(_)
                | Error::JoinError(_)
                | Error::SemaphoreError(_)
                | Error::HashError(_)
                | Error::RequestError(_)
            ),
        }
    }
}

impl From<Error> for sdk::api::error::ApiError {
    fn from(value: Error) -> Self {
        if let Error::Common(err) = value {
            return err.into();
        }

        let message = 'msg: {
            Cow::Borrowed(match value {
                _ if value.is_fatal() => "Internal Server Error",
                // TODO: at least say if it's a database error, for now
                Error::DbError(_) => "Database Error",
                Error::AuthTokenError(_) => "Auth Token Parse Error",
                Error::IOError(_) => "IO Error",
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
            Error::HashError(_)             => ApiErrorCode::HashError,
            Error::RequestError(_)          => ApiErrorCode::RequestError,
            Error::IOError(_)               => ApiErrorCode::IOError,

            | Error::Utf8ParseError(_)
            | Error::Utf8CheckError(_)      => ApiErrorCode::Utf8ParseError,

            Error::AuthTokenError(_)        => ApiErrorCode::AuthTokenError,
            Error::CaptchaError(_)          => ApiErrorCode::InvalidCaptcha,
            Error::SearchError(_)           => ApiErrorCode::SearchError,

            Error::Common(_)                => unreachable!(),
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
                SqlState::FOREIGN_KEY_VIOLATION => return CommonError::NotFound.into(),
                SqlState::CHECK_VIOLATION => return CommonError::BadRequest.into(),
                SqlState::UNIQUE_VIOLATION => return CommonError::BadRequest.into(),
                _ => {}
            }
        }

        Error::DbError(e)
    }
}
