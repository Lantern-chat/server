pub mod totp;

use aes_gcm_siv::aead::generic_array::{typenum::Unsigned, GenericArray};
use aes_gcm_siv::{
    aead::{AeadCore, AeadInPlace, Error},
    Aes256GcmSiv, Key, KeyInit, KeySizeUser, Nonce,
};

use rand::{CryptoRng, Rng};

const AD: &[u8] = b"Lantern";
const NUM_BACKUPS: usize = 8;
const KEY_LENGTH: usize = 32; // 256-bit key
const SALT_LENGTH: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct MFA {
    pub salt: [u8; SALT_LENGTH],
    pub key: [u8; KEY_LENGTH],
    pub backups: [u64; NUM_BACKUPS],
}

const MFA_LENGTH: usize = size_of::<MFA>();

// total length of the final encrypted data
const ENCRYPTED_LENGTH: usize = MFA_LENGTH
    + <Aes256GcmSiv as AeadCore>::CiphertextOverhead::USIZE
    + <Aes256GcmSiv as AeadCore>::TagSize::USIZE;

// assert that the key length matches the key size, and that the MFA struct is packed
const _: () = const {
    if KEY_LENGTH != <Aes256GcmSiv as KeySizeUser>::KeySize::USIZE {
        panic!("Key length mismatch in MFA");
    }

    if MFA_LENGTH
        != (size_of::<[u8; SALT_LENGTH]>() + size_of::<[u64; NUM_BACKUPS]>() + size_of::<[u8; KEY_LENGTH]>())
    {
        panic!("Length mismatch in MFA, possible padding");
    }
};

impl MFA {
    pub fn generate(mut rng: impl CryptoRng + Rng) -> Self {
        MFA {
            salt: {
                let mut salt = [0; 16];
                rng.fill_bytes(salt.as_mut_slice());
                salt
            },
            key: {
                let mut key = [0; KEY_LENGTH];
                rng.fill_bytes(key.as_mut_slice());
                key
            },
            backups: rng.gen(),
        }
    }

    pub fn encrypt(
        &self,
        key: &Key<Aes256GcmSiv>,
        nonce: &Nonce,
        password: &str,
    ) -> Result<[u8; ENCRYPTED_LENGTH], Error> {
        let key = compute_2fa_key(key, password, &self.salt);

        // initialize with the data we're going to encrypt
        let mut buf = {
            let mut tmp = [0u8; ENCRYPTED_LENGTH];
            tmp[..MFA_LENGTH].copy_from_slice(unsafe { std::mem::transmute::<&MFA, &[u8; MFA_LENGTH]>(self) });
            tmp
        };

        // do encryption and produce tag signature, excluding the salt at the beginning
        let tag = Aes256GcmSiv::new(&key) //
            .encrypt_in_place_detached(nonce, AD, &mut buf[SALT_LENGTH..MFA_LENGTH])?;

        // copy tag to end of output buffer
        buf[MFA_LENGTH..].copy_from_slice(&tag);

        Ok(buf)
    }

    pub fn decrypt(key: &Key<Aes256GcmSiv>, nonce: &Nonce, password: &str, data: &[u8]) -> Result<MFA, Error> {
        assert_eq!(data.len(), ENCRYPTED_LENGTH, "Length mismatch in MFA::decrypt");

        // compute key using the salt at the beginning of the data
        let key = compute_2fa_key(key, password, &data[..SALT_LENGTH]);

        // split tag and data
        let tag = GenericArray::from_slice(&data[MFA_LENGTH..]);
        let mut data = {
            let mut tmp = [0u8; MFA_LENGTH];
            tmp.copy_from_slice(&data[..MFA_LENGTH]);
            tmp
        };

        // decrypt data in place, excluding the salt at the beginning
        Aes256GcmSiv::new(&key).decrypt_in_place_detached(nonce, AD, &mut data[SALT_LENGTH..], tag)?;

        Ok(unsafe { std::mem::transmute::<[u8; MFA_LENGTH], MFA>(data) })
    }

    pub fn totp<const DIGITS: usize>(&self) -> totp::TOTP<DIGITS> {
        totp::TOTP::new(&self.key)
    }
}

/// Hashes the user password with Argon2 and XORs it with the MFA key
pub fn compute_2fa_key(mfa_key: &Key<Aes256GcmSiv>, password: &str, salt: &[u8]) -> Key<Aes256GcmSiv> {
    use rustcrypto_argon2::{Algorithm, Argon2, AssociatedData, ParamsBuilder, Version};
    use std::sync::LazyLock;

    static HASHER: LazyLock<Argon2<'static>> = LazyLock::new(|| {
        let params = ParamsBuilder::new()
            .data(AssociatedData::new(b"LanternMFA").unwrap())
            .m_cost(1024 * 12)
            .p_cost(1)
            .t_cost(3)
            .output_len(KEY_LENGTH)
            .build()
            .expect("Invalid Argon2 configuration");

        Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
    });

    let mut key = [0u8; KEY_LENGTH];

    HASHER.hash_password_into(password.as_bytes(), salt, &mut key).expect("Failed to hash password");

    // XOR keys together
    for (key, mfa_key) in key.iter_mut().zip(mfa_key) {
        *key ^= mfa_key;
    }

    key.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mfa() {
        println!("{}", ENCRYPTED_LENGTH);

        let mut rng = rand::thread_rng();

        let mfa = MFA::generate(&mut rng);
        let key = GenericArray::from_slice(b"01010101010101010101010101010101");
        let nonce = GenericArray::from_slice(b"012345678910");
        let password = "HelloWorld";

        let encrypted = mfa.encrypt(key, nonce, password).unwrap();
        let decrypted = MFA::decrypt(key, nonce, password, &encrypted).unwrap();

        assert_eq!(mfa, decrypted);

        println!("{:?}", encrypted);
    }
}
