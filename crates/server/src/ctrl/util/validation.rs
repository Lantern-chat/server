use regex::{Regex, RegexBuilder};

use crate::{config::Config, ctrl::Error};

lazy_static::lazy_static! {
    static ref EMAIL_REGEX: Regex = Regex::new(r#"^[^@\s]{1,64}@[^@\s]+\.[^.@\s]+$"#).unwrap();
    static ref USERNAME_REGEX: Regex = Regex::new(r#"^[^\s].*[^\s]$"#).unwrap();
    static ref PASSWORD_REGEX: Regex = Regex::new(r#"[^\P{L}]|\p{N}"#).unwrap();

    pub static ref USERNAME_SANITIZE_REGEX: Regex = Regex::new(r#"\s+"#).unwrap();
}

pub fn validate_username(config: &Config, username: &str) -> Result<(), Error> {
    if !config.account.username_len.contains(&username.len()) || !USERNAME_REGEX.is_match(username) {
        return Err(Error::InvalidUsername);
    }

    Ok(())
}

pub fn validate_password(config: &Config, password: &str) -> Result<(), Error> {
    if !config.account.password_len.contains(&password.len()) || !PASSWORD_REGEX.is_match(password) {
        return Err(Error::InvalidPassword);
    }

    Ok(())
}

pub fn validate_email(email: &str) -> Result<(), Error> {
    if email.len() > 320 || !EMAIL_REGEX.is_match(email) {
        return Err(Error::InvalidEmail);
    }

    Ok(())
}
