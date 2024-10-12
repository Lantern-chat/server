use crate::prelude::*;

use std::sync::LazyLock;

use rustcrypto_argon2 as argon2;

use argon2::{
    password_hash::{self, ParamsString, PasswordHash, PasswordVerifier, SaltString},
    Algorithm, Argon2, AssociatedData, ParamsBuilder, Version,
};

const ALGORITHM: Algorithm = Algorithm::Argon2id;
const VERSION: Version = Version::V0x13;

static HASHER: LazyLock<Argon2<'static>> = LazyLock::new(|| {
    let params = ParamsBuilder::new()
        .data(AssociatedData::new(b"Lantern").unwrap())
        .m_cost(super::MEM_COST)
        .p_cost(super::PARALLELISM)
        .t_cost(super::TIME_COST)
        .output_len(super::OUTPUT_LEN)
        .build()
        .expect("Invalid Argon2 configuration");

    Argon2::new(ALGORITHM, VERSION, params)
});

static PARAM_STRING: LazyLock<ParamsString> = LazyLock::new(|| ParamsString::try_from(HASHER.params()).unwrap());

pub async fn verify_password(state: &ServerState, passhash: &str, password: &str) -> Result<bool, Error> {
    // NOTE: Given how expensive it can be to compute an argon2 hash,
    // this only allows a given number to process at once.
    let _permit = state.mem_semaphore.acquire_many(HASHER.params().m_cost()).await?;

    // SAFETY: These are only used within the following spawn_blocking block
    let passhash: &'static str = unsafe { std::mem::transmute(passhash) };
    let password: &'static str = unsafe { std::mem::transmute(password) };

    let verified = tokio::task::spawn_blocking(|| {
        let hash = PasswordHash::new(passhash)?;

        match HASHER.verify_password(password.as_bytes(), &hash) {
            Ok(()) => Ok(true),
            Err(password_hash::Error::Password) => Ok(false),
            Err(e) => Err(e.into()),
        }
    })
    .await?;

    drop(_permit);

    verified
}

pub async fn hash_password(state: &ServerState, password: &str) -> Result<String, Error> {
    let _permit = state.mem_semaphore.acquire_many(HASHER.params().m_cost()).await?;

    // SAFETY: This value is only used in the below blocking future
    let password: &'static str = unsafe { std::mem::transmute(password) };

    let password_hash_task = tokio::task::spawn_blocking(move || {
        use rand::Rng;

        let salt: [u8; 16] = util::rng::crypto_thread_rng().gen();
        let mut output = [0u8; super::OUTPUT_LEN];
        HASHER.hash_password_into(password.as_bytes(), &salt, &mut output)?;

        // this is so stupid...
        let output = password_hash::Output::new(&output)?;
        let salt = SaltString::encode_b64(&salt)?;

        let passhash = PasswordHash {
            algorithm: ALGORITHM.ident(),
            version: Some(VERSION.into()),
            params: PARAM_STRING.clone(),
            salt: Some(salt.as_salt()),
            hash: Some(output),
        };

        Ok::<String, Error>(passhash.to_string())
    });

    let passhash = password_hash_task.await??;

    drop(_permit);

    Ok(passhash)
}
