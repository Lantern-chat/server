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

        AuthToken(util::rng::crypto_thread_rng().gen())
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
    pub fn from_bytes(b: &[u8]) -> Result<Self, AuthTokenFromStrError> {
        let decoded = base64::decode(&b[..Self::CHAR_LEN])?;

        match decoded.try_into() {
            Ok(inner) => Ok(AuthToken(inner)),
            Err(_) => Err(AuthTokenFromStrError::LengthError),
        }
    }
}

use db::Snowflake;

use crate::ServerState;

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

use super::Error;

pub async fn do_auth(state: &ServerState, raw_token: &[u8]) -> Result<Authorization, Error> {
    let token = AuthToken::from_bytes(raw_token)?;

    // TODO: Cache this
    let session = state
        .read_db()
        .await
        .query_opt_cached_typed(
            || {
                use db::schema::*;
                use thorn::*;

                Query::select()
                    .cols(&[Sessions::UserId, Sessions::Expires])
                    .from_table::<Sessions>()
                    .and_where(Sessions::Token.equals(Var::of(Sessions::Token)))
            },
            &[&&token.0[..]],
        )
        .await?;

    match session {
        Some(row) => {
            let expires: std::time::SystemTime = row.get(1);

            if expires <= std::time::SystemTime::now() {
                return Err(Error::NoSession);
            }

            Ok(Authorization {
                token,
                user_id: row.get(0),
            })
        }
        None => Err(Error::NoSession),
    }
}