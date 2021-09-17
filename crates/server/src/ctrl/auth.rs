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
use std::time::SystemTime;

impl FromStr for AuthToken {
    type Err = AuthTokenFromStrError;

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        // trim and check length
        s = s.trim();
        if s.len() < Self::CHAR_LEN {
            return Err(AuthTokenFromStrError::LengthError);
        }

        // decode
        let decoded = base64::decode(&s[..Self::CHAR_LEN])?;

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

use schema::Snowflake;

use crate::ServerState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Authorization {
    pub token: AuthToken,
    pub user_id: Snowflake,
    pub expires: SystemTime,
}

use super::Error;

pub async fn do_auth(state: &ServerState, raw_token: &[u8]) -> Result<Authorization, Error> {
    let token = AuthToken::from_bytes(raw_token)?;

    let auth = match state.session_cache.get(&token).await {
        Some(auth) => Some(auth),
        None => {
            let db = state.db.read.get().await?;

            let row = db
                .query_opt_cached_typed(
                    || {
                        use schema::*;
                        use thorn::*;

                        Query::select()
                            .cols(&[Sessions::UserId, Sessions::Expires])
                            .from_table::<Sessions>()
                            .and_where(Sessions::Token.equals(Var::of(Sessions::Token)))
                    },
                    &[&&token.0[..]],
                )
                .await?;

            match row {
                Some(row) => Some({
                    let auth = Authorization {
                        token,
                        user_id: row.try_get(0)?,
                        expires: row.try_get(1)?,
                    };

                    state.session_cache.set(auth).await;

                    auth
                }),
                None => None,
            }
        }
    };

    match auth {
        Some(auth) if auth.expires > SystemTime::now() => Ok(auth),
        _ => Err(Error::NoSession),
    }
}
