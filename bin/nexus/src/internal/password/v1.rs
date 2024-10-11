use crate::prelude::*;

use std::sync::LazyLock;

use rust_argon2 as argon2;

#[allow(clippy::field_reassign_with_default)]
static HASH_CONFIG: LazyLock<argon2::Config<'static>> = LazyLock::new(|| {
    let mut config = rust_argon2::Config::default();

    // OWASP recommended configuration with t=3 and 12 MiB memory.
    config.ad = b"Lantern";
    config.mem_cost = super::MEM_COST;
    config.variant = argon2::Variant::Argon2id;
    config.lanes = super::PARALLELISM;
    config.time_cost = super::TIME_COST;
    config.hash_length = super::OUTPUT_LEN;

    config
});

pub async fn verify_password(state: &ServerState, passhash: &str, password: &str) -> Result<bool, Error> {
    // NOTE: Given how expensive it can be to compute an argon2 hash,
    // this only allows a given number to process at once.
    let _permit = state.mem_semaphore.acquire_many(HASH_CONFIG.mem_cost).await?;

    // SAFETY: These are only used within the following spawn_blocking block
    let passhash: &'static str = unsafe { std::mem::transmute(passhash) };
    let password: &'static str = unsafe { std::mem::transmute(password) };

    let verified = tokio::task::spawn_blocking(|| {
        let config = &*HASH_CONFIG;

        argon2::verify_encoded_ext(passhash, password.as_bytes(), config.secret, config.ad)
    })
    .await??;

    drop(_permit);

    Ok(verified)
}

pub async fn hash_password(state: &ServerState, password: &str) -> Result<String, Error> {
    let _permit = state.mem_semaphore.acquire_many(HASH_CONFIG.mem_cost).await?;

    // SAFETY: This value is only used in the below blocking future
    let password: &'static str = unsafe { std::mem::transmute(password) };

    let password_hash_task = tokio::task::spawn_blocking(move || {
        use rand::Rng;

        let salt: [u8; 16] = util::rng::crypto_thread_rng().gen();

        argon2::hash_encoded(password.as_bytes(), &salt, &HASH_CONFIG)
    });

    let res = Ok(password_hash_task.await??);

    drop(_permit);

    res
}
