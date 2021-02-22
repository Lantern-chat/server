use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AuthToken(pub [u8; Self::TOKEN_LEN]);

impl fmt::Display for AuthToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.encode())
    }
}

const fn base64bytes(bytes: usize) -> usize {
    ((4 * bytes / 3) + 3) & !3
}

impl AuthToken {
    pub const TOKEN_LEN: usize = 21; // produces 31 characters of base64 exactly, no padding
    pub const CHAR_LEN: usize = base64bytes(Self::TOKEN_LEN);
}

impl AuthToken {
    /// Generate a new random auth token using a cryptographically secure random number generator
    pub fn new() -> AuthToken {
        use rand::Rng;

        AuthToken(crate::rng::crypto_thread_rng().gen())
    }

    /// Get the raw bytes of the token
    pub fn bytes(&self) -> &[u8] {
        &self.0[..]
    }

    /// Encode the auth token as a base-64 string
    pub fn encode(&self) -> String {
        base64::encode(&self.0)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuthTokenFromStrError {
    #[error("Length Error")]
    LengthError,

    #[error("Decode Error: {0}")]
    DecodeError(#[from] base64::DecodeError),
}

use std::convert::TryInto;

impl FromStr for AuthToken {
    type Err = AuthTokenFromStrError;

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        // trim and limit for performance
        s = &s.trim()[..Self::CHAR_LEN];

        // decode
        let decoded = base64::decode(s)?;

        // copy into fixed array
        match decoded.try_into() {
            Ok(inner) => Ok(AuthToken(inner)),
            Err(_) => Err(AuthTokenFromStrError::LengthError),
        }
    }
}

impl AuthToken {
    /// Attempt to parse an auth token from raw bytes
    pub fn from_bytes(mut b: &[u8]) -> Result<Self, AuthTokenFromStrError> {
        let decoded = base64::decode(&b[..Self::CHAR_LEN])?;

        match decoded.try_into() {
            Ok(inner) => Ok(AuthToken(inner)),
            Err(_) => Err(AuthTokenFromStrError::LengthError),
        }
    }
}

use headers::HeaderMapExt;
use http::StatusCode;

use crate::db::{ClientError, Snowflake};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Authorization {
    pub token: AuthToken,
    pub user_id: Snowflake,
}

impl Authorization {
    pub fn testing() -> Authorization {
        Authorization {
            token: AuthToken([0; AuthToken::TOKEN_LEN]),
            user_id: Snowflake::null(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Missing Authorization header")]
    MissingHeader,

    #[error("No Session")]
    NoSession,

    #[error("Invalid Format")]
    InvalidFormat,

    #[error("Decode Error: {0}")]
    DecodeError(#[from] base64::DecodeError),

    #[error("Client Error: {0}")]
    ClientError(#[from] ClientError),

    #[error("Auth Token Parse Error: {0}")]
    AuthTokenParseError(#[from] AuthTokenFromStrError),
}

use crate::server::ftl::*;

pub async fn authorize(route: &Route) -> Result<Authorization, AuthError> {
    const BEARER: &'static [u8] = b"Bearer ";

    let header = match route.req.headers().get("Authorization") {
        Some(header) => header.as_bytes(),
        None => return Err(AuthError::MissingHeader),
    };

    if !header.starts_with(BEARER) {
        return Err(AuthError::InvalidFormat);
    }

    let token = AuthToken::from_bytes(&header[BEARER.len()..])?;

    // TODO: Cache this
    let session = route
        .state
        .db
        .query_opt_cached(
            || "SELECT user_id, expires FROM lantern.sessions WHERE token = $1",
            &[&&token.0[..]],
        )
        .await?;

    match session {
        Some(row) => {
            let expires: std::time::SystemTime = row.get(1);

            if expires <= std::time::SystemTime::now() {
                return Err(AuthError::NoSession);
            }

            Ok(Authorization {
                token,
                user_id: row.get(0),
            })
        }
        None => Err(AuthError::NoSession),
    }
}

impl Reply for AuthError {
    fn into_response(self) -> Response {
        match self {
            // TODO: Maybe don't include decode error?
            AuthError::ClientError(_) | AuthError::DecodeError(_) => {
                log::error!("Auth Error: {}", self);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
            _ => self
                .to_string()
                .with_status(StatusCode::BAD_REQUEST)
                .into_response(),
        }
    }
}
