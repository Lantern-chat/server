use db::{ClientError, Snowflake};

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

pub async fn do_login(
    state: ServerState,
    id: Snowflake,
    now: std::time::SystemTime,
) -> Result<Session, Error> {
    let token = AuthToken::new();

    let expires = now + state.config.login_session_duration;

    state
        .db
        .write
        .execute_cached(
            || "INSERT INTO lantern.sessions (token, user_id, expires) VALUES ($1, $2, $3)",
            &[&&token.0[..], &id, &expires],
        )
        .await?;

    Ok(Session {
        auth: token.encode(),
        expires: time::OffsetDateTime::from(expires).format(time::Format::Rfc3339),
    })
}
