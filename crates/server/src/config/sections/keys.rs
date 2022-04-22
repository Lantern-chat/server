use aes::{cipher::BlockCipherKey, Aes128, Aes256};
use schema::auth::BotTokenKey;

#[derive(Debug, Serialize, Deserialize)]
pub struct Keys {
    /// File encryption key
    #[serde(with = "super::util::hex_key")]
    pub file_key: BlockCipherKey<Aes256>,

    /// Multi-factor authentication encryption key
    #[serde(with = "super::util::hex_key")]
    pub mfa_key: BlockCipherKey<Aes256>,

    /// Some snowflakes are encrypted as a form of reversable obfuscation.
    #[serde(with = "super::util::hex_key")]
    pub sf_key: BlockCipherKey<Aes128>,

    #[serde(with = "super::util::hex_key")]
    pub bt_key: BotTokenKey,
}

// NOTE: When not present, keys will be filled in with random bytes
// which will be written back to the file for persistence
impl Default for Keys {
    fn default() -> Keys {
        Keys {
            file_key: util::rng::gen_crypto_bytes().into(),
            mfa_key: util::rng::gen_crypto_bytes().into(),
            sf_key: util::rng::gen_crypto_bytes().into(),
            bt_key: util::rng::gen_crypto_bytes().into(),
        }
    }
}
