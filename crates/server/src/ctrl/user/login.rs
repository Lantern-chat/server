use db::Snowflake;

use crate::{
    ctrl::{auth::AuthToken, Error},
    ServerState,
};

#[derive(Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
}

use models::Session;

use super::register::{hash_config, EMAIL_REGEX};

// TODO: Determine if I should give any feedback at all or
// just say catchall "invalid username/email/password"
pub async fn login(state: ServerState, form: LoginForm) -> Result<Session, Error> {
    if !EMAIL_REGEX.is_match(&form.email) {
        return Err(Error::InvalidCredentials);
    }

    let user = state
        .read_db()
        .await
        .query_opt_cached_typed(
            || {
                use db::schema::*;
                use thorn::*;

                Query::select()
                    .from_table::<Users>()
                    .cols(&[Users::Id, Users::Email, Users::Passhash, Users::DeletedAt])
                    .and_where(Users::Email.equals(Var::of(Users::Email)))
            },
            &[&form.email],
        )
        .await?;

    let user = match user {
        Some(user) => user,
        None => return Err(Error::InvalidCredentials),
    };

    let id: Snowflake = user.get(0);
    let passhash: String = user.get(2);
    let deleted_at: Option<time::PrimitiveDateTime> = user.get(3);

    if deleted_at.is_some() {
        return Err(Error::InvalidCredentials);
    }

    // NOTE: Given how expensive it can be to compute an argon2 hash,
    // this only allows a given number to process at once.
    let permit = state.hashing_semaphore.acquire().await?;

    let verified = tokio::task::spawn_blocking(move || {
        let config = hash_config();
        argon2::verify_encoded_ext(
            &passhash,
            form.password.as_bytes(),
            config.secret,
            config.ad,
        )
    })
    .await??;

    drop(permit);

    if !verified {
        return Err(Error::InvalidCredentials);
    }

    do_login(state, id, std::time::SystemTime::now()).await
}

pub async fn do_login(
    state: ServerState,
    id: Snowflake,
    now: std::time::SystemTime,
) -> Result<Session, Error> {
    let token = AuthToken::new();

    let expires = now + state.config.login_session_duration;

    state
        .write_db()
        .await
        .execute_cached_typed(
            || {
                use db::schema::*;
                use thorn::*;

                Query::insert()
                    .into::<Sessions>()
                    .cols(&[Sessions::Token, Sessions::UserId, Sessions::Expires])
                    .values(vec![
                        Var::of(Sessions::Token),
                        Var::of(Sessions::UserId),
                        Var::of(Sessions::Expires),
                    ])
            },
            &[&&token.0[..], &id, &expires],
        )
        .await?;

    Ok(Session {
        auth: token.encode(),
        expires: time::OffsetDateTime::from(expires).format(time::Format::Rfc3339),
    })
}