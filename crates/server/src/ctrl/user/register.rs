use std::{net::SocketAddr, sync::Arc, time::SystemTime};

use db::{Snowflake, SnowflakeExt};

use crate::{ctrl::Error, ServerState};

#[derive(Clone, Deserialize)]
pub struct RegisterForm {
    pub email: String,
    pub username: String,
    pub password: String,
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

use models::Session;

use regex::{Regex, RegexBuilder};

lazy_static::lazy_static! {
    pub static ref EMAIL_REGEX: Regex = Regex::new(r#"^[^@\s]{1,64}@[^@\s]+\.[^.@\s]+$"#).unwrap();
    pub static ref USERNAME_REGEX: Regex = Regex::new(r#"^[^\s].*[^\s]$"#).unwrap();

    static ref PASSWORD_REGEX: Regex = Regex::new(r#"[^\P{L}]|\p{N}"#).unwrap();

    static ref USERNAME_SANITIZE_REGEX: Regex = Regex::new(r#"\s+"#).unwrap();
}

pub async fn register_user(
    state: ServerState,
    addr: SocketAddr,
    mut form: RegisterForm,
) -> Result<Session, Error> {
    if !state.config.username_len.contains(&form.username.len())
        || !USERNAME_REGEX.is_match(&form.username)
    {
        return Err(Error::InvalidUsername);
    }

    if !state.config.password_len.contains(&form.password.len())
        || !PASSWORD_REGEX.is_match(&form.password)
    {
        return Err(Error::InvalidPassword);
    }

    if form.email.len() > 320 || !EMAIL_REGEX.is_match(&form.email) {
        return Err(Error::InvalidEmail);
    }

    let dob = time::Date::try_from_ymd(form.year, form.month + 1, form.day + 1)?;
    let now = SystemTime::now();

    if !crate::util::time::is_of_age(state.config.min_user_age_in_years as i64, now, dob) {
        return Err(Error::InsufficientAge);
    }

    let existing = state
        .read_db()
        .await
        .query_opt_cached_typed(
            || {
                use db::schema::*;
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

    let password = std::mem::replace(&mut form.password, String::new());

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

    state
        .write_db()
        .await
        .execute_cached_typed(
            || {
                use db::schema::*;
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
