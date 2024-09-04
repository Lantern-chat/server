use std::net::SocketAddr;

use crate::prelude::*;

use sdk::{
    api::commands::user::UserLoginForm,
    models::{ElevationLevel, Session, UserFlags},
};

use crate::internal::{
    login::do_login,
    mfa::{process_2fa, validate_2fa_token, ProvidedMfa},
    password::verify_password,
};

pub async fn login(
    state: ServerState,
    addr: SocketAddr,
    form: &Archived<UserLoginForm>,
) -> Result<Session, Error> {
    if form.password.len() < 8 {
        return Err(Error::InvalidCredentials);
    }

    if !schema::validation::validate_email(&form.email) {
        return Err(Error::InvalidEmail);
    }

    let mut totp = None;

    // early validation
    if let Some(token) = form.totp.as_ref() {
        if token.is_empty() {
            totp = None;
        } else {
            validate_2fa_token(token)?;

            totp = Some(token);
        }
    }

    #[rustfmt::skip]
    let user = state.db.read.get().await?.query_opt2(schema::sql! {
        SELECT
            Users.Id        AS @Id,
            Users.Flags     AS @Flags,
            Users.Passhash  AS @Passhash,
            Users.Mfa       AS @Mfa
        FROM  LiveUsers AS Users
        WHERE Users.Email = #{&form.email as Users::Email}
    }).await?;

    let Some(user) = user else {
        return Err(Error::InvalidCredentials);
    };

    let user_id: UserId = user.id()?;
    let flags = UserFlags::from_bits_truncate(user.flags()?);
    let passhash = user.passhash()?;
    let mfa: Option<&[u8]> = user.mfa()?;

    match flags.elevation() {
        // System user flat out cannot log in. Pretend it doesn't exist.
        ElevationLevel::System => return Err(Error::NotFound),

        // don't allow staff to login without 2FA set up
        ElevationLevel::Staff if mfa.is_none() => {
            log::error!("Staff user {user_id} tried to login without 2FA enabled");

            return Err(Error::NotFound);
        }
        _ => {}
    }

    if flags.contains(UserFlags::BANNED) {
        return Err(Error::Banned);
    }

    // early check, before any work is done
    if mfa.is_some() && form.totp.is_none() {
        return Err(Error::TOTPRequired);
    }

    if !verify_password(&state, passhash, &form.password).await? {
        return Err(Error::InvalidCredentials);
    }

    if let Some(mfa) = mfa {
        if !process_2fa(
            &state,
            user_id,
            ProvidedMfa::Encrypted(mfa),
            &form.password,
            totp.unwrap(),
        )
        .await?
        {
            return Err(Error::InvalidCredentials);
        }
    }

    do_login(state, addr, user_id).await
}
