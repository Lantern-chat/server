use aes_gcm_siv::{aead::Aead, Aes256GcmSiv, KeyInit, Nonce};

use sdk::models::Snowflake;

#[inline]
fn nonce_from_user_id(user_id: Snowflake) -> Nonce {
    let mut nonce = [0u8; 12]; // 96-bit
    let id_bytes: [u8; 8] = user_id.to_u64().to_be_bytes();
    nonce[0..8].copy_from_slice(&id_bytes[0..8]);
    nonce[8..12].copy_from_slice(&id_bytes[2..6]);
    Nonce::from(nonce)
}

pub fn encrypt_user_message(key: &[u8], user_id: Snowflake, plaintext: &[u8]) -> Vec<u8> {
    let gcm = Aes256GcmSiv::new_from_slice(key).unwrap();
    gcm.encrypt(&nonce_from_user_id(user_id), plaintext)
        .expect("Unable to encrypt")
}

pub fn decrypt_user_message(key: &[u8], user_id: Snowflake, ciphertext: &[u8]) -> Result<Vec<u8>, ()> {
    let gcm = Aes256GcmSiv::new_from_slice(key).unwrap();
    gcm.decrypt(&nonce_from_user_id(user_id), ciphertext).map_err(|_| ()) // can error from invalid signatures, so allow it through.
}
