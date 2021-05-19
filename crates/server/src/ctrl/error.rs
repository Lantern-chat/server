use std::borrow::Cow;

use db::ClientError;
use ftl::StatusCode;

use crate::web::gateway::event::EncodingError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // FATAL ERRORS
    // TODO: Add backtraces when https://github.com/dtolnay/thiserror/pull/131 lands
    #[error("Database Error {0}")]
    DbError(#[from] ClientError),
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

    // NON-FATAL ERRORS
    #[error("User Already Exists")]
    UserAlreadyExists,

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
    InvalidDate(#[from] time::error::ComponentRange),

    #[error("Invalid Message Content")]
    InvalidContent,

    #[error("Invalid Name")]
    InvalidName,

    #[error("Invalid Room Topic")]
    InvalidTopic,

    #[error("Missing Upload-Metadata Header")]
    MissingUploadMetadataHeader,

    #[error("Missing Authorization Header")]
    MissingAuthorizationHeader,

    #[error("No Session")]
    NoSession,

    #[error("Invalid Auth Format")]
    InvalidAuthFormat,

    #[error("Header Parse Error")]
    HeaderParseError,

    #[error("Missing filename")]
    MissingFilename,

    #[error("Missing filetype")]
    MissingFiletype,

    #[error("Auth Token Parse Error: {0}")]
    AuthTokenParseError(#[from] super::auth::AuthTokenFromStrError),

    #[error("Decode Error: {0}")]
    DecodeError(#[from] base64::DecodeError),
}

impl From<db::PgError> for Error {
    fn from(err: db::PgError) -> Error {
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
        )
    }

    pub fn http_status(&self) -> StatusCode {
        if self.is_fatal() {
            return StatusCode::INTERNAL_SERVER_ERROR;
        }

        match self {
            Error::NoSession => StatusCode::FORBIDDEN,
            _ => StatusCode::BAD_REQUEST,
        }
    }

    pub fn code(&self) -> u16 {
        // TODO: Decide on actual error codes
        self.http_status().as_u16()
    }

    pub fn format(&self) -> Cow<'static, str> {
        Cow::Borrowed(match self {
            // TODO: at least say if it's a database error, for now
            Error::DbError(_) => "Database Error",
            Error::AuthTokenParseError(_) => "Auth Token Parse Error",
            Error::DecodeError(_) => "Base64 Decode Error",

            _ if self.is_fatal() => "Internal Server Error",
            _ => return self.to_string().into(),
        })
    }
}
