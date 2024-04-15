use aes_gcm_siv::{aead::Aead, Aes256GcmSiv, Key, KeyInit, Nonce};

use crate::prelude::*;

#[inline]
pub fn nonce_from_user_id(user_id: UserId) -> Nonce {
    let mut nonce = [0u8; 12]; // 96-bit
    let id_bytes: [u8; 8] = user_id.to_u64().to_be_bytes();
    nonce[0..8].copy_from_slice(&id_bytes[0..8]);
    nonce[8..12].copy_from_slice(&id_bytes[2..6]);
    Nonce::from(nonce)
}

pub fn encrypt_user_message(key: &Key<Aes256GcmSiv>, user_id: UserId, plaintext: &[u8]) -> Vec<u8> {
    Aes256GcmSiv::new(key).encrypt(&nonce_from_user_id(user_id), plaintext).expect("Unable to encrypt")
}

/// Returns None in cases of invalid or corrupted input
pub fn decrypt_user_message(key: &Key<Aes256GcmSiv>, user_id: UserId, ciphertext: &[u8]) -> Option<Vec<u8>> {
    Aes256GcmSiv::new(key).decrypt(&nonce_from_user_id(user_id), ciphertext).ok()
}
