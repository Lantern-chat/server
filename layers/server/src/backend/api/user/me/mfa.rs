use std::time::{Duration, SystemTime};

use mfa_totp::{totp::TOTP6, MFA};
use sdk::api::commands::user::{Added2FA, Confirm2FAForm, Enable2FAForm, Remove2FAForm};
use sdk::models::{ElevationLevel, Snowflake, UserFlags};

use crate::backend::util::encrypt::nonce_from_user_id;
use crate::{Error, ServerState};

use super::login::ProvidedMfa;

pub async fn enable_2fa(state: ServerState, user_id: Snowflake, form: Enable2FAForm) -> Result<Added2FA, Error> {
    let config = state.config.load_full();

    if !config.account.password_len.contains(&form.password.len()) {
        return Err(Error::InvalidCredentials);
    }

    let _verified = state
        .services
        .hcaptcha
        .verify(crate::backend::services::hcaptcha::HCaptchaParameters {
            secret: &config.services.hcaptcha_secret,
            sitekey: Some(&config.services.hcaptcha_sitekey),
            response: &form.token,
            remoteip: None, // TODO
        })
        .await?;

    #[rustfmt::skip]
    let user = state.db.read.get().await?.query_one2(schema::sql! {
        SELECT
            Users.Mfa IS NOT NULL AS @HasMFA,
            Users.Email     AS @Email,
            Users.Passhash  AS @Passhash
        FROM Users WHERE #{&user_id as Users::Id}
    }).await?;

    // if the user already has 2FA enabled, it must be disabled first
    if user.has_mfa()? {
        return Err(Error::Conflict);
    }

    if !super::login::verify_password(&state, user.passhash()?, &form.password).await? {
        return Err(Error::InvalidCredentials);
    }

    let email = user.email()?;

    let mfa = MFA::generate(util::rng::crypto_thread_rng());

    let Ok(ref encrypted_mfa) = mfa.encrypt(&config.keys.mfa_key, &nonce_from_user_id(user_id), &form.password)
    else {
        return Err(Error::InternalErrorStatic("Encryption Error"));
    };

    let expires = SystemTime::now() + Duration::from_secs(config.account.mfa_pending_time.max(1) as u64);

    // Upsert pending MFA
    #[rustfmt::skip]
    state.db.write.get().await?.execute2(schema::sql! {
        INSERT INTO MfaPending (UserId, Expires, Mfa) VALUES (
            #{&user_id          as Users::Id},
            #{&expires          as MfaPending::Expires},
            #{&encrypted_mfa    as MfaPending::Mfa}
        ) ON CONFLICT DO UPDATE MfaPending SET (Expires, Mfa) = (
            #{&expires          as MfaPending::Expires},
            #{&encrypted_mfa    as MfaPending::Mfa}
        )
    }).await?;

    Ok(Added2FA {
        // create URL for addition to an authenticator app
        url: TOTP6::new(&mfa.key).url(email, &config.general.server_name),
        // encode each 64-bit backup code
        backup: Vec::from_iter(mfa.backups.iter().map(|code| {
            // NOTE: Little-Endian is used intentionally here
            base32::encode(base32::Alphabet::Crockford, &code.to_le_bytes())
        })),
    })
}

pub async fn confirm_2fa(state: ServerState, user_id: Snowflake, form: Confirm2FAForm) -> Result<(), Error> {
    if form.totp.len() != 6 {
        return Err(Error::TOTPRequired);
    }

    let mut db = state.db.write.get().await?;

    let t = db.transaction().await?;

    #[rustfmt::skip]
    let user = t.query_one2(schema::sql! {
        SELECT
             MfaPending.Mfa AS @Mfa,
            (MfaPending.Expires <= now()) AS @Expired
         FROM MfaPending
        WHERE MfaPending.UserId = #{&user_id as Users::Id}
    }).await?;

    if user.expired()? {
        return Err(Error::NotFound);
    }

    let encrypted_mfa = user.mfa()?;

    // NOTE: Backup codes aren't actually an option here,
    // given the check at the start of this function,
    // so we don't have to worry about writing to the database.
    if !super::login::process_2fa(
        &state,
        user_id,
        ProvidedMfa::Encrypted(encrypted_mfa),
        &form.password,
        &form.totp,
    )
    .await?
    {
        return Err(Error::InvalidCredentials);
    }

    tokio::try_join!(
        t.execute2(schema::sql! {
            DELETE FROM MfaPending WHERE MfaPending.UserId = #{&user_id as Users::Id}
        }),
        t.execute2(schema::sql! {
            UPDATE Users SET (Flags, Mfa) = (
                Users.Flags | {UserFlags::MFA_ENABLED.bits()},
                #{&encrypted_mfa as Users::Mfa}
            ) WHERE Users.Id = #{&user_id as Users::Id}
        })
    )?;

    t.commit().await?;

    Ok(())
}

pub async fn remove_2fa(state: ServerState, user_id: Snowflake, form: Remove2FAForm) -> Result<(), Error> {
    if !state.config().account.password_len.contains(&form.password.len()) {
        return Err(Error::InvalidCredentials);
    }

    super::login::validate_2fa_token(&form.totp)?;

    #[rustfmt::skip]
    let user = state.db.read.get().await?.query_one2(schema::sql! {
        SELECT
            Users.Flags     AS @Flags,
            Users.Passhash  AS @Passhash,
            Users.Mfa       AS @Mfa
        FROM Users WHERE Users.Id = #{&user_id as Users::Id}
    }).await?;

    let flags = UserFlags::from_bits_truncate(user.flags()?);

    // these roles are not allowed to remove 2FA
    if let ElevationLevel::System | ElevationLevel::Staff = flags.elevation() {
        return Err(Error::Unauthorized);
    }

    let Some(encrypted_mfa) = user.mfa()? else {
        return Err(Error::NotFound);
    };

    let passhash = user.passhash()?;

    if !super::login::verify_password(&state, passhash, &form.password).await? {
        return Err(Error::InvalidCredentials);
    }

    if !super::login::process_2fa(
        &state,
        user_id,
        ProvidedMfa::Encrypted(encrypted_mfa),
        &form.password,
        &form.totp,
    )
    .await?
    {
        return Err(Error::InvalidCredentials);
    }

    #[rustfmt::skip]
    state.db.write.get().await?.execute2(schema::sql! {
        UPDATE Users SET (Mfa, Flags) = (NULL, Users.Flags & ~{UserFlags::MFA_ENABLED.bits()})
        WHERE Users.Id = #{&user_id as Users::Id}
    }).await?;

    Ok(())
}
