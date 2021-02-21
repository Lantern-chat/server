use rand::Rng;
use std::{sync::Arc, time::SystemTime};

use http::StatusCode;

use crate::{
    db::{ClientError, Snowflake},
    server::{
        ftl::{
            body::{content_length_limit, form, BodyDeserializeError},
            rate_limit::RateLimitKey,
            reply,
        },
        routes::api::util::time::is_of_age,
        ServerState,
    },
};

use super::{auth::AuthToken, Reply, Route};

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
    // 10KB max form size
    if let Some(err) = content_length_limit(&route, 1024 * 10) {
        return err.into_response();
    }

    let form = match form::<RegisterForm>(&mut route).await {
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
    pub static ref EMAIL_REGEX: Regex = Regex::new(r#"^[^@\s]+@[^@\s]+\.[^.@\s]+$"#).unwrap();
    pub static ref USERNAME_REGEX: Regex = Regex::new(r#"^[^\s].{1,62}[^\s]$"#).unwrap();

    static ref PASSWORD_REGEX: Regex = Regex::new(r#"\P{L}|\p{N}"#).unwrap();

    static ref USERNAME_SANITIZE_REGEX: Regex = Regex::new(r#"\s+"#).unwrap();
}

// TODO: Set these in server config
const MIN_AGE: i64 = 18;
const MIN_PASSWORD_LEN: usize = 8;
const MAX_USERNAME_LEN: usize = 64;
const MIN_USERNAME_LEN: usize = 3;

use super::login::{do_login, Session};

async fn register_user(
    state: ServerState,
    mut form: RegisterForm,
) -> Result<Session, RegisterError> {
    if !USERNAME_REGEX.is_match(&form.username) {
        return Err(RegisterError::InvalidUsername);
    }

    if !PASSWORD_REGEX.is_match(&form.password) {
        return Err(RegisterError::InvalidPassword);
    }

    if !EMAIL_REGEX.is_match(&form.email) {
        return Err(RegisterError::InvalidEmail);
    }

    let dob = time::Date::try_from_ymd(form.year, form.month + 1, form.day + 1)?;
    let now = SystemTime::now();

    if !is_of_age(MIN_AGE, now, dob) {
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

    let id =
        Snowflake::at_ms((now - time::OffsetDateTime::unix_epoch()).whole_milliseconds() as u128);

    let password = std::mem::replace(&mut form.password, String::new());

    // fire this off while we sanitize the username
    let password_hash_task = tokio::task::spawn_blocking(move || {
        let config = hash_config();
        let salt: [u8; 16] = crate::rng::crypto_thread_rng().gen();
        argon2::hash_encoded(password.as_bytes(), &salt, &config)
    });

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
