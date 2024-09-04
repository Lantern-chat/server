use std::{net::SocketAddr, time::SystemTime};

use crate::prelude::*;

use crate::internal::{login::do_login, password::hash_password};
use crate::services::hcaptcha::HCaptchaParameters;

use sdk::api::commands::all::UserRegister;
use sdk::models::Session;

pub async fn register_user(
    state: ServerState,
    addr: SocketAddr,
    cmd: &Archived<UserRegister>,
) -> Result<Session, Error> {
    let form = &cmd.body;

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

    let dob = Timestamp::from(form.dob).date();

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

    let passhash = hash_password(&state, form.password.as_str()).await?;

    let user_id = state.sf.gen();

    #[rustfmt::skip]
    state.db.write.get().await?.execute2(schema::sql! {
        CALL .register_user(
            #{&user_id          as Users::Id},
            #{&form.username    as Users::Username},
            #{&form.email       as Users::Email},
            #{&passhash         as Users::Passhash},
            #{&dob              as Users::Dob}
        )
    }).await?;

    do_login(state, addr, user_id).await
}
