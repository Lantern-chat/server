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
    pub fn new() -> AuthToken {
        use rand::Rng;

        AuthToken(crate::rng::crypto_thread_rng().gen())
    }

    pub fn bytes(&self) -> &[u8] {
        &self.0[..]
    }

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
