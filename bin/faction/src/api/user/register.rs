use std::{net::SocketAddr, time::SystemTime};

use crate::{prelude::*, services::hcaptcha::HCaptchaParameters};

use sdk::{api::commands::user::UserRegisterForm, models::Session};

pub async fn register_user(
    state: ServerState,
    addr: SocketAddr,
    mut form: UserRegisterForm,
) -> Result<Session, Error> {
    //if cfg!(debug_assertions) {
    //    return Err(Error::TemporarilyDisabled);
    //}

    let config = state.config();

    if !schema::validation::validate_email(&form.email) {
        return Err(Error::InvalidEmail);
    }
    if !schema::validation::validate_username(&form.username, config.shared.username_length.clone()) {
        return Err(Error::InvalidUsername);
    }
    if !schema::validation::validate_password(&form.password, config.shared.password_length.clone()) {
        return Err(Error::InvalidPassword);
    }

    let dob = form.dob.try_into()?;

    let now = SystemTime::now();

    if !util::time::is_of_age(config.shared.minimum_age as i32, now, dob) {
        return Err(Error::InsufficientAge);
    }

    let _verified = state
        .services
        .hcaptcha
        .verify(HCaptchaParameters {
            secret: &config.shared.hcaptcha_secret,
            sitekey: Some(&config.shared.hcaptcha_sitekey),
            response: &form.token,
            remoteip: None, // TODO
        })
        .await?;

    let read_db = state.db.read.get().await?;

    #[rustfmt::skip]
    let existing = read_db.query_opt2(schema::sql! {
        SELECT FROM Users WHERE Users.Email = #{&form.email as Users::Email}
    }).await?;

    if existing.is_some() {
        return Err(Error::AlreadyExists);
    }

    let password = std::mem::take(&mut form.password);

    let _permit = state.mem_semaphore.acquire_many(hash_memory_cost()).await?;

    // fire this off while we sanitize the username
    let password_hash_task = tokio::task::spawn_blocking(move || {
        use rand::Rng;

        let config = hash_config();
        let salt: [u8; 16] = util::rng::crypto_thread_rng().gen();
        let res = argon2::hash_encoded(password.as_bytes(), &salt, &config);

        res
    });

    let id = state.sf.gen();
    let passhash = password_hash_task.await??;

    drop(_permit);

    #[rustfmt::skip]
    state.db.write.get().await?.execute2(schema::sql! {
        CALL .register_user(
            #{&id               as Users::Id},
            #{&form.username    as Users::Username},
            #{&form.email       as Users::Email},
            #{&passhash         as Users::Passhash},
            #{&dob              as Users::Dob}
        )
    }).await?;

    super::me::login::do_login(state, addr, id, now).await
}

/// Returns the amount of memory the argon2 hash will use, in kilobytes
pub const fn hash_memory_cost() -> u32 {
    8 * 1024 // 8 MiB
}

#[allow(clippy::field_reassign_with_default)]
pub fn hash_config() -> argon2::Config<'static> {
    let mut config = argon2::Config::default();

    config.ad = b"Lantern";
    config.mem_cost = hash_memory_cost();
    config.variant = argon2::Variant::Argon2id;
    config.lanes = 1;
    config.time_cost = 3;
    config.hash_length = 24;

    config
}
