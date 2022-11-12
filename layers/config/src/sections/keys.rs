use aes::{cipher::Key, Aes128, Aes256};
use schema::auth::BotTokenKey;

type CamoKey = hmac::digest::Key<hmac::SimpleHmac<sha1::Sha1>>;

section! {
    /// NOTE: When not present, keys will be filled in with random bytes.
    pub struct Keys {
        /// File encryption key
        #[serde(with = "super::util::hex_key")]
        pub file_key: Key<Aes256> = util::rng::gen_crypto_bytes().into() => "FS_KEY" | parse_hex_key[true],

        /// Multi-factor authentication encryption key
        #[serde(with = "super::util::hex_key")]
        pub mfa_key: Key<Aes256> = util::rng::gen_crypto_bytes().into() => "MFA_KEY" | parse_hex_key[true],

        /// Some snowflakes are encrypted as a form of reversable obfuscation.
        #[serde(with = "super::util::hex_key")]
        pub sf_key: Key<Aes128> = util::rng::gen_crypto_bytes().into() => "SF_KEY" | parse_hex_key[true],

        /// Bot Token Key (padded)
        ///
        /// Used for signing bot tokens
        #[serde(with = "super::util::hex_key::loose")]
        pub bt_key: BotTokenKey = util::rng::gen_crypto_bytes().into() => "BT_KEY" | parse_hex_key[false],

        /// Signing key for camo proxies (padded)
        #[serde(with = "super::util::hex_key::loose")]
        pub camo_key: CamoKey = util::rng::gen_crypto_bytes().into() => "CAMO_KEY" | parse_hex_key[false],
    }
}

pub fn parse_hex_key<const N: usize>(value: &str, strict: bool) -> [u8; N] {
    let mut key = [0; N];
    super::util::hex_key::parse_hex_key_inner(&mut key, value, strict).expect("Error parsing key");
    key
}
