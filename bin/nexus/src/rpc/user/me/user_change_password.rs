use mfa_totp::MFA;

use sdk::api::commands::user::ChangePasswordForm;

use crate::internal::{
    mfa::{process_2fa, ProvidedMfa},
    password::{hash_password, verify_password},
};

use crate::{prelude::*, util::encrypt::nonce_from_user_id};

pub async fn change_password(
    state: ServerState,
    auth: Authorization,
    form: &Archived<ChangePasswordForm>,
) -> Result<(), Error> {
    let config = state.config_full();

    if !config.shared.password_length.contains(&form.current.len()) {
        return Err(Error::InvalidCredentials);
    };

    if !schema::validation::validate_password(&form.new, config.shared.password_length.clone()) {
        return Err(Error::InvalidPassword);
    }

    #[rustfmt::skip]
    let user = state.db.read.get().await?.query_one2(schema::sql! {
        SELECT
            Users.Passhash  AS @Passhash,
            Users.Mfa       AS @Mfa
        FROM  Users
        WHERE Users.Id = #{auth.user_id_ref() as Users::Id}
    }).await?;

    let passhash = user.passhash()?;
    let encrypted_mfa: Option<&[u8]> = user.mfa()?;

    if encrypted_mfa.is_some() && form.totp.is_none() {
        return Err(Error::TOTPRequired);
    }

    if !verify_password(&state, passhash, &form.current).await? {
        return Err(Error::InvalidCredentials);
    }

    let mut new_mfa = None;

    // if MFA is enabled, it needs to be verified and re-encrypted with the new password
    if let (Some(token), Some(mfa)) = (form.totp.as_deref(), encrypted_mfa) {
        let mfa_key = config.local.keys.mfa_key;
        let nonce = nonce_from_user_id(auth.user_id());

        let Ok(mfa) = MFA::decrypt(&mfa_key, &nonce, &form.current, mfa) else {
            return Err(Error::InternalErrorStatic("Decrypt Error"));
        };

        if !process_2fa(&state, auth.user_id(), ProvidedMfa::Plain(&mfa), &form.current, token).await? {
            return Err(Error::InvalidCredentials);
        }

        new_mfa = match mfa.encrypt(&mfa_key, &nonce, &form.new) {
            Ok(mfa) => Some(mfa),
            Err(_) => return Err(Error::InternalErrorStatic("Encrypt Error")),
        };
    }

    // hash the new password after MFA verification and modification
    let passhash = hash_password(&state, &form.new).await?;

    #[rustfmt::skip]
    state.db.write.get().await?.execute2(schema::sql! {
        UPDATE Users SET (Passhash, Mfa) = (
            #{&passhash as Users::Passhash},
            #{&new_mfa  as Users::Mfa}
        ) WHERE Users.Id = #{auth.user_id_ref() as Users::Id}
    }).await?;

    Ok(())
}
