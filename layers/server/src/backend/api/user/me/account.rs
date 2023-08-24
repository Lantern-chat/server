use sdk::models::Snowflake;
use smol_str::SmolStr;

use crate::{
    backend::util::validation::{validate_email, validate_password, validate_username, USERNAME_SANITIZE_REGEX},
    Authorization, Error, ServerState,
};

use super::login::ProvidedMfa;

#[derive(Deserialize)]
pub struct ModifyAccountForm {
    pub password: SmolStr,
    #[serde(default)]
    pub totp: Option<SmolStr>,

    #[serde(default)]
    pub new_username: Option<SmolStr>,
    #[serde(default)]
    pub new_email: Option<SmolStr>,
}

pub async fn modify_account(
    state: ServerState,
    auth: Authorization,
    mut form: ModifyAccountForm,
) -> Result<(), Error> {
    let mut num_fields = 0;

    let config = state.config();

    if let Some(ref username) = form.new_username {
        validate_username(&config, username)?;
        num_fields += 1;
    }

    if let Some(ref email) = form.new_email {
        validate_email(email)?;
        num_fields += 1;
    }

    if num_fields == 0 {
        return Err(Error::BadRequest);
    }

    let db = state.db.read.get().await?;

    #[rustfmt::skip]
    let user = db.query_one2(schema::sql! {
        SELECT
            Users.Id        AS @UserId,
            Users.Username  AS @Username,
            Users.Passhash  AS @Passhash,
            Users.Mfa       AS @Mfa
        FROM Users WHERE Users.Id = #{&auth.user_id as Users::Id}
    }).await?;

    let user_id: Snowflake = auth.user_id;
    let old_username: &str = user.username()?;
    let passhash: &str = user.passhash()?;
    let mfa: Option<&[u8]> = user.mfa()?;

    if let Some(ref new_username) = form.new_username {
        if new_username == old_username && num_fields == 1 {
            return Ok(()); // changing username to same value is a no-op...
        }
    }

    if !super::login::verify_password(&state, passhash, &form.password).await? {
        return Err(Error::InvalidCredentials);
    }

    if let Some(mfa) = mfa {
        let Some(token) = form.totp else {
            return Err(Error::TOTPRequired);
        };

        if !super::login::process_2fa(&state, user_id, ProvidedMfa::Encrypted(mfa), &form.password, &token).await?
        {
            return Err(Error::InvalidCredentials);
        }
    }

    let mut u = None;
    let e = form.new_email;

    if let Some(ref username) = form.new_username {
        let new_username = USERNAME_SANITIZE_REGEX.replace_all(username, " ");

        if old_username == new_username && num_fields == 1 {
            // TODO: Move this up?
            return Ok(()); // stop here, even though time was wasted
        }

        u = Some(new_username);
    }

    drop(user); // referenced data from `user` row no longer needed, last used borrow of username above.

    // let db = state.db.write.get().await?;

    // db.execute_cached_typed(
    //     || {
    //         use schema::*;
    //         use thorn::*;

    //         Query::call(schema::update_user::call(
    //             Var::of(Users::Id),
    //             Var::of(Users::Username),
    //             Var::of(Users::Email),
    //             Var::of(Users::Passhash),
    //         ))
    //     },
    //     &[&auth.user_id, &u, &e, &p],
    // )
    // .await?;

    Ok(())
}
