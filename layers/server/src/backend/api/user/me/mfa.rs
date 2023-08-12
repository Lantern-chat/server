use std::time::{Duration, SystemTime};

use sdk::api::commands::user::{Added2FA, Confirm2FAForm, Enable2FAForm, Remove2FAForm};
use sdk::models::{ElevationLevel, Snowflake, UserFlags};
use totp::TOTP6;

use crate::{backend::util::encrypt::encrypt_user_message, Error, ServerState};

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
            Users.MfaSecret IS NOT NULL AS @HasMFA,
            Users.Email     AS @Email,
            Users.Passhash  AS @Passhash
        FROM Users WHERE #{&user_id as Users::Id}
    }).await?;

    if user.has_mfa()? {
        return Err(Error::Conflict);
    }

    if !super::login::verify_password(&state, user.passhash()?, form.password).await? {
        return Err(Error::InvalidCredentials);
    }

    let email = user.email()?;

    let (secret, backups) = {
        use rand::Rng;
        let mut rng = util::rng::crypto_thread_rng();

        (
            rng.gen::<[u8; 32]>(), // 256-bit key and backup codes
            (0..config.account.num_mfa_backups)
                .flat_map(|_| rng.gen::<u64>().to_be_bytes())
                .collect::<Vec<u8>>(),
        )
    };

    let encrypted_secret = encrypt_user_message(&config.keys.mfa_key, user_id, &secret);
    let encrypted_backups = encrypt_user_message(&config.keys.mfa_key, user_id, &backups);

    let expires = SystemTime::now() + Duration::from_secs(config.account.mfa_pending_time.max(1) as u64);

    // Upsert pending MFA
    #[rustfmt::skip]
    state.db.write.get().await?.execute2(schema::sql! {
        INSERT INTO MfaPending (UserId, Expires, MfaSecret, MfaBackup) VALUES (
            #{&user_id           as Users::Id},
            #{&expires           as MfaPending::Expires},
            #{&encrypted_secret  as MfaPending::MfaSecret},
            #{&encrypted_backups as MfaPending::MfaBackup}
        ) ON CONFLICT DO UPDATE MfaPending SET (Expires, MfaSecret, MfaBackup) = (
            #{&expires           as MfaPending::Expires},
            #{&encrypted_secret  as MfaPending::MfaSecret},
            #{&encrypted_backups as MfaPending::MfaBackup}
        )
    }).await?;

    Ok(Added2FA {
        // create URL for addition to an authenticator app
        url: TOTP6::new(&secret).url(email, &config.general.server_name),
        // encode each 64-bit backup code
        backup: backups.chunks_exact(8).map(|code| base32::encode(base32::Alphabet::Crockford, code)).collect(),
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
             MfaPending.MfaSecret AS @MfaSecret,
             MfaPending.MfaBackup AS @MfaBackup,
            (MfaPending.Expires < now()) AS @Expired
         FROM MfaPending
        WHERE MfaPending.UserId = #{&user_id as Users::Id}
    }).await?;

    if user.expired()? {
        return Err(Error::NotFound);
    }

    let mfa_secret = user.mfa_secret()?;
    let mfa_backup = user.mfa_backup()?;

    // NOTE: Backup codes aren't actually an option here,
    // given the check at the start of this function,
    // so we don't have to worry about writing to the database.
    if !super::login::process_2fa(&state, user_id, mfa_secret, mfa_backup, &form.totp).await? {
        return Err(Error::InvalidCredentials);
    }

    tokio::try_join!(
        t.execute2(schema::sql! {
            DELETE FROM MfaPending WHERE MfaPending.UserId = #{&user_id as Users::Id}
        }),
        t.execute2(schema::sql! {
            UPDATE Users SET (Flags, MfaSecret, MfaBackup) = (
                Users.Flags | {UserFlags::MFA_ENABLED.bits()},
                #{&mfa_secret as Users::MfaSecret},
                #{&mfa_backup as Users::MfaBackup}
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
            Users.MfaSecret AS @MfaSecret,
            Users.MfaBackup AS @MfaBackup
        FROM Users WHERE Users.Id = #{&user_id as Users::Id}
    }).await?;

    let flags = UserFlags::from_bits_truncate(user.flags()?);

    // these roles are not allowed to remove 2FA
    if let ElevationLevel::System | ElevationLevel::Staff = flags.elevation() {
        return Err(Error::Unauthorized);
    }

    let Some(mfa_secret) = user.mfa_secret()? else {
        return Err(Error::NotFound);
    };

    let passhash = user.passhash()?;
    let mfa_backup = user.mfa_backup()?;

    if !super::login::verify_password(&state, passhash, form.password).await? {
        return Err(Error::InvalidCredentials);
    }

    if !super::login::process_2fa(&state, user_id, mfa_secret, mfa_backup, &form.totp).await? {
        return Err(Error::InvalidCredentials);
    }

    #[rustfmt::skip]
    state.db.write.get().await?.execute2(schema::sql! {
        UPDATE Users SET (MfaSecret, MfaBackup, Flags) = (
            NULL, NULL, Users.Flags & ~{UserFlags::MFA_ENABLED.bits()}
        ) WHERE Users.Id = #{&user_id as Users::Id}
    }).await?;

    Ok(())
}
