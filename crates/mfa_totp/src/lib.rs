pub mod totp;

use aes_gcm_siv::aead::generic_array::{typenum::Unsigned, GenericArray};
use aes_gcm_siv::{
    aead::{AeadCore, AeadInPlace, Error},
    Aes256GcmSiv, Key, KeyInit, Nonce,
};

use rand::{CryptoRng, Rng};

const AD: &[u8] = b"Lantern";
const NUM_BACKUPS: usize = 8;
const KEY_LENGTH: usize = 256 / 8; // 256-bit key as bytes

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct MFA {
    pub backups: [u64; NUM_BACKUPS],
    pub key: [u8; KEY_LENGTH],
}

const MFA_LENGTH: usize = std::mem::size_of::<MFA>();

const ENCRYPTED_LENGTH: usize = MFA_LENGTH
    + <Aes256GcmSiv as AeadCore>::CiphertextOverhead::USIZE
    + <Aes256GcmSiv as AeadCore>::TagSize::USIZE;

impl MFA {
    pub fn generate(mut rng: impl CryptoRng + Rng) -> Self {
        MFA {
            backups: rng.gen(),
            key: {
                let mut key = [0; KEY_LENGTH];
                rng.fill_bytes(key.as_mut_slice());
                key
            },
        }
    }

    pub fn encrypt(
        &self,
        key: &Key<Aes256GcmSiv>,
        nonce: &Nonce,
        password: &str,
    ) -> Result<[u8; ENCRYPTED_LENGTH], Error> {
        let key = compute_2fa_key(key, password);

        // initialize with the data we're going to encrypt
        let mut buf: [u8; ENCRYPTED_LENGTH] = {
            let mut tmp = [0; ENCRYPTED_LENGTH];
            tmp[..MFA_LENGTH].copy_from_slice(unsafe { std::mem::transmute::<&MFA, &[u8; MFA_LENGTH]>(self) });
            tmp
        };

        // do encryption and produce tag signature
        let tag = Aes256GcmSiv::new(&key) //
            .encrypt_in_place_detached(nonce, AD, &mut buf[..MFA_LENGTH])?;

        // copy tag to end of output buffer
        buf[MFA_LENGTH..].copy_from_slice(&tag);

        Ok(buf)
    }

    pub fn decrypt(key: &Key<Aes256GcmSiv>, nonce: &Nonce, password: &str, data: &[u8]) -> Result<MFA, Error> {
        assert_eq!(data.len(), ENCRYPTED_LENGTH, "Length mismatch in MFA::decrypt");

        let key = compute_2fa_key(key, password);

        // split tag and data
        let tag = GenericArray::from_slice(&data[MFA_LENGTH..]);
        let mut data: [u8; MFA_LENGTH] = {
            let mut tmp = [0; MFA_LENGTH];
            tmp.copy_from_slice(&data[..MFA_LENGTH]);
            tmp
        };

        Aes256GcmSiv::new(&key).decrypt_in_place_detached(nonce, AD, &mut data, tag)?;

        Ok(unsafe { std::mem::transmute::<[u8; 96], MFA>(data) })
    }

    pub fn totp<const DIGITS: usize>(&self) -> totp::TOTP<DIGITS> {
        totp::TOTP::new(&self.key)
    }
}

/// Hashes the user password with SHA3-256, then XORs it with the MFA key
pub fn compute_2fa_key(mfa_key: &Key<Aes256GcmSiv>, password: &str) -> Key<Aes256GcmSiv> {
    use sha3::{digest::FixedOutput, Digest, Sha3_256};

    let mut key: Key<Aes256GcmSiv> = {
        let mut passhash = <Sha3_256 as Digest>::new();
        passhash.update(password);
        passhash.finalize_fixed()
    };

    // XOR keys together
    for (key, mfa_key) in key.iter_mut().zip(mfa_key) {
        *key ^= mfa_key;
    }

    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mfa() {
        println!("{}", ENCRYPTED_LENGTH);

        let mut rng = rand::thread_rng();

        let mfa = MFA {
            backups: rng.gen(),
            key: rng.gen(),
        };
        let key = GenericArray::from_slice(b"01010101010101010101010101010101");
        let nonce = GenericArray::from_slice(b"012345678910");
        let password = "HelloWorld";

        let encrypted = mfa.encrypt(key, nonce, password).unwrap();
        let decrypted = MFA::decrypt(key, nonce, password, &encrypted).unwrap();

        assert_eq!(mfa, decrypted);

        println!("{:?}", encrypted);
    }
}
