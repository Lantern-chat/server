use std::fmt;

use sdk::models::{BearerToken, BotToken, InvalidAuthToken};

pub use sdk::models::AuthToken;

mod bot;

// (4 * bytes) / 3, rounded up to nearest multiple of 4 for padding
#[allow(dead_code)]
const fn base64bytes(bytes: usize) -> usize {
    ((4 * bytes / 3) + 3) & !3
}

const BEARER_BYTES_LEN: usize = 21;
const BOT_BYTES_LEN: usize = SplitBotToken::SPLIT_BOT_TOKEN_SIZE; // should be 36

pub use bot::{BotTokenKey, SplitBotToken};

pub type UserToken = [u8; BEARER_BYTES_LEN];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, rkyv::Archive, rkyv::Serialize)]
pub enum RawAuthToken {
    Bearer(UserToken),
    Bot(SplitBotToken),
}

static_assertions::const_assert_eq!(base64bytes(BEARER_BYTES_LEN), BearerToken::LEN);
static_assertions::const_assert_eq!(base64bytes(BOT_BYTES_LEN), BotToken::LEN);

#[derive(Debug, thiserror::Error)]
pub enum AuthTokenError {
    #[error("Length Error")]
    LengthError,

    #[error("Decode Error: {0}")]
    DecodeError(#[from] base64::DecodeError),

    #[error("Invalid Auth Token")]
    InvalidAuthToken(#[from] InvalidAuthToken),
}

use rand_core::RngCore;

impl RawAuthToken {
    pub fn bearer(mut rng: impl RngCore) -> Self {
        let mut bytes = [0; BEARER_BYTES_LEN];
        rng.fill_bytes(&mut bytes);
        RawAuthToken::Bearer(bytes)
    }

    //pub fn bot(mut rng: impl RngCore) -> Self {
    //    let mut bytes = [0; BOT_BYTES_LEN];
    //    rng.fill_bytes(&mut bytes);
    //    RawAuthToken::Bot(bytes)
    //}

    pub fn from_header(value: &str) -> Result<Self, AuthTokenError> {
        AuthToken::from_header(value)?.try_into()
    }
}

use base64::engine::{general_purpose::STANDARD_NO_PAD, Engine};

impl From<RawAuthToken> for AuthToken {
    fn from(token: RawAuthToken) -> AuthToken {
        // SAFETY: sizes are asserted above and in debug
        match token {
            RawAuthToken::Bearer(bytes) => unsafe {
                let mut s = BearerToken::zeroized();
                if Ok(BearerToken::LEN) != STANDARD_NO_PAD.encode_slice(bytes, s.as_bytes_mut()) {
                    unreachable!("Could not encode auth token to base64");
                };
                AuthToken::Bearer(s)
            },
            RawAuthToken::Bot(token) => AuthToken::Bot(token.format()),
        }
    }
}

impl TryFrom<AuthToken> for RawAuthToken {
    type Error = AuthTokenError;

    fn try_from(token: AuthToken) -> Result<RawAuthToken, AuthTokenError> {
        Ok(match token {
            AuthToken::Bearer(token) => {
                let mut bytes = [0; BEARER_BYTES_LEN];
                STANDARD_NO_PAD.decode_slice_unchecked(token, &mut bytes)?;
                RawAuthToken::Bearer(bytes)
            }
            AuthToken::Bot(token) => RawAuthToken::Bot(token.parse()?),
        })
    }
}

impl TryFrom<&[u8]> for RawAuthToken {
    type Error = AuthTokenError;

    fn try_from(bytes: &[u8]) -> Result<RawAuthToken, AuthTokenError> {
        if bytes.len() == BEARER_BYTES_LEN {
            return Ok(RawAuthToken::Bearer({
                let mut buf = [0; BEARER_BYTES_LEN];
                buf.copy_from_slice(bytes);
                buf
            }));
        }

        if bytes.len() == BOT_BYTES_LEN {
            return Ok(RawAuthToken::Bot(SplitBotToken::try_from(bytes)?));
        }

        Err(InvalidAuthToken.into())
    }
}

impl std::str::FromStr for RawAuthToken {
    type Err = AuthTokenError;

    fn from_str(s: &str) -> Result<Self, AuthTokenError> {
        // parse and decode
        AuthToken::from_str(s)?.try_into()
    }
}

impl fmt::Display for RawAuthToken {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        AuthToken::from(*self).fmt(f)
    }
}
