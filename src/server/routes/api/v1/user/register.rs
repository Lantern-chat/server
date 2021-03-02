use rand::Rng;
use std::{sync::Arc, time::SystemTime};

use http::StatusCode;

use crate::{
    db::{ClientError, Snowflake},
    server::{routes::api::util::time::is_of_age, ServerState},
};

use crate::server::ftl::*;
use crate::server::routes::api::auth::AuthToken;

#[derive(Clone, Deserialize)]
pub struct RegisterForm {
    email: String,
    username: String,
    password: String,
    year: i32,
    month: u8,
    day: u8,
}

pub async fn register(mut route: Route) -> impl Reply {
    let form = match body::any::<RegisterForm>(&mut route).await {
        Ok(form) => form,
        Err(e) => return e.into_response(),
    };

    match register_user(route.state, form).await {
        Ok(ref session) => reply::json(session).into_response(),
        Err(e) => match e {
            RegisterError::ClientError(_)
            | RegisterError::JoinError(_)
            | RegisterError::PasswordHashError(_) => {
                log::error!("Register Error: {}", e);

                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
            _ => e
                .to_string()
                .with_status(StatusCode::BAD_REQUEST)
                .into_response(),
        },
    }
}

#[derive(thiserror::Error, Debug)]
enum RegisterError {
    #[error("Invalid Email Address")]
    InvalidEmail,

    #[error("Invalid Username")]
    InvalidUsername,

    #[error("Invalid Password")]
    InvalidPassword,

    #[error("Email already registered")]
    AlreadyExists,

    #[error("Database Error {0}")]
    ClientError(#[from] ClientError),

    #[error("Invalid Date of Birth")]
    InvalidDob(#[from] time::error::ComponentRange),

    #[error("Too Young")]
    TooYoungError,

    #[error("Join Error {0}")]
    JoinError(#[from] tokio::task::JoinError),

    #[error("Password Hash Error {0}")]
    PasswordHashError(#[from] argon2::Error),
}

use regex::{Regex, RegexBuilder};

lazy_static::lazy_static! {
    pub static ref EMAIL_REGEX: Regex = Regex::new(r#"^[^@\s]{1,64}@[^@\s]+\.[^.@\s]+$"#).unwrap();
    pub static ref USERNAME_REGEX: Regex = Regex::new(r#"^[^\s].*[^\s]$"#).unwrap();

    static ref PASSWORD_REGEX: Regex = Regex::new(r#"\P{L}|\p{N}"#).unwrap();

    static ref USERNAME_SANITIZE_REGEX: Regex = Regex::new(r#"\s+"#).unwrap();
}

use super::login::{do_login, Session};

async fn register_user(
    state: ServerState,
    mut form: RegisterForm,
) -> Result<Session, RegisterError> {
    if !state.config.username_len.contains(&form.username.len())
        || !USERNAME_REGEX.is_match(&form.username)
    {
        return Err(RegisterError::InvalidUsername);
    }

    if !state.config.password_len.contains(&form.password.len())
        || !PASSWORD_REGEX.is_match(&form.password)
    {
        return Err(RegisterError::InvalidPassword);
    }

    if form.email.len() > 320 || !EMAIL_REGEX.is_match(&form.email) {
        return Err(RegisterError::InvalidEmail);
    }

    let dob = time::Date::try_from_ymd(form.year, form.month + 1, form.day + 1)?;
    let now = SystemTime::now();

    if !is_of_age(state.config.min_user_age_in_years as i64, now, dob) {
        return Err(RegisterError::TooYoungError);
    }

    let existing = state
        .db
        .query_opt_cached(
            || "SELECT id FROM lantern.users WHERE email=$1",
            &[&form.email],
        )
        .await?;

    if existing.is_some() {
        return Err(RegisterError::AlreadyExists);
    }

    let password = std::mem::replace(&mut form.password, String::new());

    // fire this off while we sanitize the username
    let password_hash_task = tokio::task::spawn_blocking(move || {
        let config = hash_config();
        let salt: [u8; 16] = crate::rng::crypto_thread_rng().gen();
        argon2::hash_encoded(password.as_bytes(), &salt, &config)
    });

    let id = Snowflake::at(now);
    let username = USERNAME_SANITIZE_REGEX.replace_all(&form.username, " ");

    let password_hash = password_hash_task.await??;

    state
        .db
        .execute_cached(
            || "CALL lantern.register_user($1, $2, $3, $4, $5)",
            &[&id, &username, &form.email, &password_hash, &dob],
        )
        .await?;

    Ok(do_login(state, id, now).await?)
}

pub fn hash_config() -> argon2::Config<'static> {
    let mut config = argon2::Config::default();

    config.ad = b"Lantern";
    config.variant = argon2::Variant::Argon2i;
    config.lanes = 1;
    config.time_cost = 6;
    config.thread_mode = argon2::ThreadMode::Sequential;
    config.hash_length = 24;

    config
}
