use std::{net::SocketAddr, sync::Arc, time::SystemTime};

use schema::{Snowflake, SnowflakeExt};
use smol_str::SmolStr;

use crate::{services::hcaptcha::HCaptchaParameters, util::validation::*, Error, State};

use sdk::{api::commands::user::UserRegisterForm, models::Session};

pub async fn register_user(
    state: State,
    addr: SocketAddr,
    mut form: UserRegisterForm,
) -> Result<Session, Error> {
    //if cfg!(debug_assertions) {
    //    return Err(Error::TemporarilyDisabled);
    //}

    validate_username(&state.config, &form.username)?;
    validate_password(&state.config, &form.password)?;
    validate_email(&form.email)?;

    let dob = form.dob.try_into()?;

    let now = SystemTime::now();

    if !util::time::is_of_age(state.config.account.min_age as i32, now, dob) {
        return Err(Error::InsufficientAge);
    }

    let _verified = state
        .services
        .hcaptcha
        .verify(HCaptchaParameters {
            secret: &state.config.services.hcaptcha_secret,
            sitekey: Some(&state.config.services.hcaptcha_sitekey),
            response: &form.token,
            ..HCaptchaParameters::default()
        })
        .await?;

    let read_db = state.db.read.get().await?;

    let existing = read_db
        .query_opt_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .from_table::<Users>()
                    .and_where(Users::Email.equals(Var::of(Users::Email)))
            },
            &[&form.email],
        )
        .await?;

    if existing.is_some() {
        return Err(Error::AlreadyExists);
    }

    let password = std::mem::take(&mut form.password);

    // NOTE: Given how expensive it can be to compute an argon2 hash,
    // this only allows a given number to process at once.
    let permit = state.hashing_semaphore.acquire().await?;

    // fire this off while we sanitize the username
    let password_hash_task = tokio::task::spawn_blocking(move || {
        use rand::Rng;

        let config = hash_config();
        let salt: [u8; 16] = util::rng::crypto_thread_rng().gen();
        let res = argon2::hash_encoded(password.as_bytes(), &salt, &config);

        res
    });

    let id = Snowflake::at(now);
    let username = USERNAME_SANITIZE_REGEX.replace_all(&form.username, " ");

    let password_hash = password_hash_task.await??;

    drop(permit);

    let write_db = state.db.write.get().await?;

    write_db
        .execute_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::call(Call::custom("lantern.register_user").args((
                    Var::of(Users::Id),
                    Var::of(Users::Username),
                    Var::of(Users::Email),
                    Var::of(Users::Passhash),
                    Var::of(Users::Dob),
                )))
            },
            &[&id, &username, &form.email, &password_hash, &dob],
        )
        .await?;

    super::me::login::do_login(state, addr, id, now).await
}

pub fn hash_config() -> argon2::Config<'static> {
    let mut config = argon2::Config::default();

    config.ad = b"Lantern";
    config.mem_cost = 8 * 1024; // 8 MiB
    config.variant = argon2::Variant::Argon2id;
    config.lanes = 1;
    config.time_cost = 3;
    config.thread_mode = argon2::ThreadMode::Sequential;
    config.hash_length = 24;

    config
}
