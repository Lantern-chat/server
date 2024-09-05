use crate::prelude::*;

use std::sync::LazyLock;

/// Treat as constant, the configuration for the Argon2 hashing algorithm.
#[allow(clippy::field_reassign_with_default)]
fn hash_config() -> argon2::Config<'static> {
    let mut config = argon2::Config::default();

    config.ad = b"Lantern";
    config.mem_cost = 8 * 1024; // 8 MiB;
    config.variant = argon2::Variant::Argon2id;
    config.lanes = 1;
    config.time_cost = 3;
    config.hash_length = 24;

    config
}

pub static HASH_CONFIG: LazyLock<argon2::Config<'static>> = LazyLock::new(hash_config);

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

    // fire this off while we sanitize the username
    let password_hash_task = tokio::task::spawn_blocking(move || {
        use rand::Rng;

        let salt: [u8; 16] = util::rng::crypto_thread_rng().gen();
        let res = argon2::hash_encoded(password.as_bytes(), &salt, &HASH_CONFIG);

        res
    });

    let res = Ok(password_hash_task.await??);

    drop(_permit);

    res
}
