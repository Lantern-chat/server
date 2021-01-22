use std::sync::Arc;

use warp::{
    body::json,
    hyper::{Server, StatusCode},
    reject::Reject,
    Filter, Rejection, Reply,
};

use crate::{
    db::{ClientError, Snowflake},
    server::{auth::AuthToken, rate::RateLimitKey, routes::api::ApiError, ServerState},
};

#[derive(Clone, Deserialize)]
pub struct RegisterForm {
    email: String,
    username: String,
    password: String,
    year: i32,
    month: u8,
    day: u8,
}

pub fn register(
    state: Arc<ServerState>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::post()
        .map(move || state.clone())
        .and(warp::body::form::<RegisterForm>())
        .and_then(|state: Arc<ServerState>, form: RegisterForm| async move {
            match register_user(state, form).await {
                Ok(token) => Ok::<_, Rejection>(warp::reply::with_status(
                    warp::reply::json(&token.as_str()),
                    StatusCode::OK,
                )),
                Err(ref e) => match e {
                    RegisterError::ClientError(e_inner) => {
                        log::error!("{} while Registering: {}", e, e_inner);
                        Ok(warp::reply::with_status(
                            warp::reply::json(&ApiError {
                                code: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                                message: e.to_string(),
                            }),
                            StatusCode::INTERNAL_SERVER_ERROR,
                        ))
                    }
                    _ => Ok(warp::reply::with_status(
                        warp::reply::json(&ApiError {
                            code: StatusCode::BAD_REQUEST.as_u16(),
                            message: e.to_string(),
                        }),
                        StatusCode::BAD_REQUEST,
                    )),
                },
            }
        })
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
    #[error("Database Error")]
    ClientError(#[from] ClientError),
    #[error("Invalid Date of Birth")]
    InvalidDob(#[from] time::error::ComponentRange),
    #[error("Too Young")]
    TooYoungError,
}

use regex::Regex;

lazy_static::lazy_static! {
    static ref EMAIL_REGEX: Regex = Regex::new(r#"^[^@\s]+@[^@\s]+\.[^.@\s]+$"#).unwrap();
}

// TODO: Set these in server config
const MIN_AGE: i64 = 13;
const MIN_PASSWORD_LEN: usize = 8;
const MIN_USERNAME_LEN: usize = 3;

async fn register_user(
    state: Arc<ServerState>,
    form: RegisterForm,
) -> Result<AuthToken, RegisterError> {
    // Order these tests by complexity for faster failures
    if form.username.len() < MIN_USERNAME_LEN {
        return Err(RegisterError::InvalidUsername);
    }

    if form.password.len() < MIN_PASSWORD_LEN
        || form.password.chars().find(|c| !c.is_alphabetic()).is_none()
    {
        return Err(RegisterError::InvalidPassword);
    }

    if !EMAIL_REGEX.is_match(&form.email) {
        return Err(RegisterError::InvalidEmail);
    }

    let dob = time::Date::try_from_ymd(form.year, form.month, form.day)?;
    let now = time::OffsetDateTime::now_utc();
    let today = now.date();
    let diff = today - dob;

    let mut days = diff.whole_days();
    // rough approximiation, if it's less than this, it'll be less than the exact
    if days < MIN_AGE * 365 {
        return Err(RegisterError::TooYoungError);
    } else {
        let mut years = 0;
        let mut year = today.year();
        days -= today.ordinal() as i64; // go to start of this year
        loop {
            year -= 1;
            days -= time::days_in_year(year) as i64;
            if days < 0 || years >= MIN_AGE {
                break;
            }
            years += 1;
        }

        if years < MIN_AGE {
            return Err(RegisterError::TooYoungError);
        }
    }

    let existing = state
        .db
        .query_opt_cached(
            || "SELECT (id) FROM lantern.users WHERE email=$1",
            &[&form.email],
        )
        .await?;

    if existing.is_some() {
        return Err(RegisterError::AlreadyExists);
    }

    let id =
        Snowflake::at_ms((now - time::OffsetDateTime::unix_epoch()).whole_milliseconds() as u128);

    Ok(AuthToken([0; AuthToken::TOKEN_LEN]))
}
