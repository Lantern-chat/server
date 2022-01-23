use futures::FutureExt;
use sdk::models::Snowflake;
use smol_str::SmolStr;
use thorn::pg::ToSql;

use crate::{
    ctrl::{
        util::validation::{validate_email, validate_password, validate_username, USERNAME_SANITIZE_REGEX},
        Error,
    },
    web::auth::Authorization,
    ServerState,
};

use crate::ctrl::user::{me::login::process_2fa, register::hash_config};

#[derive(Deserialize)]
pub struct ModifyAccountForm {
    pub current_password: SmolStr,

    #[serde(default)]
    pub totp: Option<SmolStr>,

    #[serde(default)]
    pub new_username: Option<SmolStr>,
    #[serde(default)]
    pub new_password: Option<SmolStr>,
    #[serde(default)]
    pub new_email: Option<SmolStr>,
}

pub async fn modify_account(
    state: ServerState,
    auth: Authorization,
    mut form: ModifyAccountForm,
) -> Result<(), Error> {
    let mut num_fields = 0;

    if let Some(ref username) = form.new_username {
        validate_username(&state.config, username)?;
        num_fields += 1;
    }

    if let Some(ref password) = form.new_password {
        validate_password(&state.config, password)?;
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

    let user = db
        .query_opt_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .from_table::<Users>()
                    .cols(&[
                        Users::Id,
                        Users::Username,
                        Users::Passhash,
                        Users::MfaSecret,
                        Users::MfaBackup,
                    ])
                    .and_where(Users::Id.equals(Var::of(Users::Id)))
            },
            &[&auth.user_id],
        )
        .await?;

    let user = match user {
        Some(user) => user,
        None => return Err(Error::InvalidCredentials),
    };

    let user_id: Snowflake = user.try_get(0)?;
    let old_username: &str = user.try_get(1)?;
    let passhash: &str = user.try_get(2)?;
    let secret: Option<&[u8]> = user.try_get(3)?;
    let backup: Option<&[u8]> = user.try_get(4)?;

    if secret.is_some() != backup.is_some() {
        return Err(Error::InternalErrorStatic("Secret/Backup Mismatch!"));
    }

    if let Some(ref new_username) = form.new_username {
        if new_username == old_username && num_fields == 1 {
            return Ok(()); // changing username to same value is a no-op...
        }
    }

    let verified = {
        // NOTE: Given how expensive it can be to compute an argon2 hash,
        // this only allows a given number to process at once.
        let permit = state.hashing_semaphore.acquire().await?;

        // SAFETY: This is only used within the following spawn_blocking block,
        // but will remain alive until `drop(user)` below.
        let passhash: &'static str = unsafe { std::mem::transmute(passhash) };

        let password = form.current_password;
        let verified = tokio::task::spawn_blocking(move || {
            let config = hash_config();
            argon2::verify_encoded_ext(passhash, password.as_bytes(), config.secret, config.ad)
        })
        .await??;

        drop(permit);

        verified
    };

    if !verified {
        return Err(Error::InvalidCredentials);
    }

    if let (Some(secret), Some(backup)) = (secret, backup) {
        match form.totp {
            None => return Err(Error::TOTPRequired),
            Some(token) => {
                if !process_2fa(&state, user_id, secret, backup, &token).await? {
                    return Err(Error::InvalidCredentials);
                }
            }
        }
    }

    let mut password_hash_task = None;

    if let Some(password) = std::mem::replace(&mut form.new_password, None) {
        let permit = state.hashing_semaphore.acquire().await?;

        password_hash_task = Some((
            permit,
            tokio::task::spawn_blocking(move || {
                use rand::Rng;

                let config = hash_config();
                let salt: [u8; 16] = util::rng::crypto_thread_rng().gen();

                let res = argon2::hash_encoded(password.as_bytes(), &salt, &config);

                res
            }),
        ));
    }

    let mut u = None;
    let mut p = None;
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

    if let Some((permit, password_hash_task)) = password_hash_task {
        let password_hash = password_hash_task.await??;

        drop(permit);

        p = Some(password_hash);
    }

    let db = state.db.write.get().await?;

    db.execute_cached_typed(
        || {
            use schema::*;
            use thorn::*;

            Query::call(Call::custom("lantern.update_user").args((
                Var::of(Users::Id),
                Var::of(Users::Username),
                Var::of(Users::Email),
                Var::of(Users::Passhash),
            )))
        },
        &[&auth.user_id, &u, &e, &p],
    )
    .await?;

    Ok(())
}
