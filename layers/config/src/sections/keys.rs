use aes::{cipher::Key, Aes128, Aes256};
use schema::auth::BotTokenKey;

section! {
    /// NOTE: When not present, keys will be filled in with random bytes.
    pub struct Keys {
        /// File encryption key
        #[serde(with = "super::util::hex_key")]
        pub file_key: Key<Aes256> = util::rng::gen_crypto_bytes().into() => "FS_KEY" | parse_hex_key,

        /// Multi-factor authentication encryption key
        #[serde(with = "super::util::hex_key")]
        pub mfa_key: Key<Aes256> = util::rng::gen_crypto_bytes().into() => "MFA_KEY" | parse_hex_key,

        /// Some snowflakes are encrypted as a form of reversable obfuscation.
        #[serde(with = "super::util::hex_key")]
        pub sf_key: Key<Aes128> = util::rng::gen_crypto_bytes().into() => "SF_KEY" | parse_hex_key,

        /// Bot Token Key
        ///
        /// Used for signing bot tokens
        #[serde(with = "super::util::hex_key")]
        pub bt_key: BotTokenKey = util::rng::gen_crypto_bytes().into() => "BT_KEY" | parse_hex_key,
    }
}

fn parse_hex_key<const N: usize>(value: &str) -> [u8; N] {
    let hex_len = value.len();

    if hex_len < 32 {
        panic!("Don't use key sizes under 128-bits for key");
    }

    let mut key = [0; N];
    if key.len() * 2 != hex_len {
        panic!("Length mismatch for {}-bit key", N * 8);
    }

    hex::decode_to_slice(value, &mut key[..hex_len / 2])
        .unwrap_or_else(|_| panic!("Invalid hexidecimal {}-bit encryption key", N * 8));

    key
}
