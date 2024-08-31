use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::{
    io::{Read, Write},
    mem::size_of,
    num::NonZeroU64,
    str::FromStr,
    time::SystemTime,
};

use base64::engine::{general_purpose::STANDARD_NO_PAD, Engine};

use hmac::{
    digest::{FixedOutput, Key},
    Mac, SimpleHmac,
};
use sha1::Sha1;

use sdk::models::{AuthToken, BotToken, InvalidAuthToken, Snowflake};

type Sha1Hmac = SimpleHmac<Sha1>;
pub type BotTokenKey = Key<Sha1Hmac>;
type HmacDigest = [u8; 20];

/// Decomposed bot token with its component parts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, rkyv::Archive, rkyv::Serialize)]
pub struct SplitBotToken {
    /// Bot identifier
    pub id: Snowflake,
    /// Seconds since UNIX epoch this token was created
    pub issued: u64,
    /// HMAC digest of the previous two fields with a private key
    pub hmac: HmacDigest,
}

impl SplitBotToken {
    pub const SPLIT_BOT_TOKEN_SIZE: usize = size_of::<Snowflake>() + size_of::<u64>() + size_of::<HmacDigest>();

    #[inline]
    pub fn to_bytes(&self) -> [u8; Self::SPLIT_BOT_TOKEN_SIZE] {
        let mut bytes = [0u8; Self::SPLIT_BOT_TOKEN_SIZE];

        unsafe {
            let mut w: &mut [u8] = &mut bytes;

            w.write_u64::<LittleEndian>(self.id.to_u64()).unwrap_unchecked();
            w.write_u64::<LittleEndian>(self.issued).unwrap_unchecked();
            w.write(&self.hmac).unwrap_unchecked();
        }

        bytes
    }

    fn token_mac(&self, key: &BotTokenKey) -> Sha1Hmac {
        let mut mac = Sha1Hmac::new(key);
        mac.update(&self.to_bytes()[0..16]);
        mac
    }

    pub fn new(key: &BotTokenKey, id: Snowflake) -> Self {
        let mut t = SplitBotToken {
            id,
            hmac: [0; 20],
            issued: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
        };

        t.hmac = t.token_mac(key).finalize_fixed().into();

        t
    }

    pub fn verify(&self, key: &BotTokenKey) -> bool {
        //token_mac(self, key).verify_slice(&self.hmac).is_ok()
        self.hmac == self.token_mac(key).finalize_fixed().as_slice()
    }

    pub fn format(&self) -> BotToken {
        let mut token = BotToken::repeat_ascii('\0');
        unsafe {
            if Ok(BotToken::LEN) != STANDARD_NO_PAD.encode_slice(self.to_bytes(), token.as_bytes_mut()) {
                unreachable!("Could not encode bot token to base64");
            };
        }

        token
    }
}

impl TryFrom<&[u8]> for SplitBotToken {
    type Error = InvalidAuthToken;

    #[inline]
    fn try_from(mut bytes: &[u8]) -> Result<SplitBotToken, InvalidAuthToken> {
        if bytes.len() != Self::SPLIT_BOT_TOKEN_SIZE {
            return Err(InvalidAuthToken);
        }

        let raw_id;
        let ts;
        let mut hmac: HmacDigest = [0; 20];

        unsafe {
            raw_id = bytes.read_u64::<LittleEndian>().unwrap_unchecked();
            ts = bytes.read_u64::<LittleEndian>().unwrap_unchecked();
            bytes.read_exact(&mut hmac).unwrap_unchecked();
        }

        let id = match NonZeroU64::new(raw_id) {
            Some(id) => Snowflake(id),
            None => return Err(InvalidAuthToken),
        };

        Ok(SplitBotToken { id, issued: ts, hmac })
    }
}

impl FromStr for SplitBotToken {
    type Err = InvalidAuthToken;

    fn from_str(s: &str) -> Result<SplitBotToken, InvalidAuthToken> {
        if s.len() != BotToken::LEN {
            return Err(InvalidAuthToken);
        }

        let mut bytes = [0; Self::SPLIT_BOT_TOKEN_SIZE];
        if STANDARD_NO_PAD.decode_slice_unchecked(s, &mut bytes).is_err() {
            return Err(InvalidAuthToken);
        }
        SplitBotToken::try_from(&bytes[..])
    }
}

impl From<SplitBotToken> for AuthToken {
    fn from(token: SplitBotToken) -> AuthToken {
        AuthToken::Bot(token.format())
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::SnowflakeExt;

//     use super::*;

//     #[test]
//     fn test_splitbottoken_bytes() {
//         let token = SplitBotToken {
//             id: Snowflake::null(),
//             issued: 0,
//             hmac: [u8::MAX; 20],
//         };

//         let bytes = token.to_bytes();

//         assert_eq!(token, SplitBotToken::try_from(&bytes[..]).unwrap());

//         println!("{}", token.format());
//     }

//     #[test]
//     fn test_new_bot_token() {
//         fn parse_key<const N: usize>(key: &str) -> [u8; N] {
//             let mut out = [0; N];
//             hex::decode_to_slice(key, &mut out[..key.len() / 2]).unwrap();
//             out
//         }

//         let key: BotTokenKey = parse_key("5f38e06b42428527d49db9513b251651").into();

//         let token = SplitBotToken::new(&key, Snowflake::now());

//         println!("{}", token.format());

//         assert!(token.verify(&key));
//     }
// }
