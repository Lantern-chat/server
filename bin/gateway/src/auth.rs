use crate::prelude::*;

use schema::auth::{RawAuthToken, SplitBotToken, UserToken};

pub trait AuthTokenExt {
    fn random_bearer() -> Self;
}

impl AuthTokenExt for RawAuthToken {
    fn random_bearer() -> Self {
        RawAuthToken::bearer(util::rng::crypto_thread_rng())
    }
}

use sdk::models::{Timestamp, UserFlags};

#[async_recursion::async_recursion]
pub async fn do_auth(state: &ServerState, token: &RawAuthToken) -> Result<Authorization, Error> {
    let auth = match state.session_cache.get(token) {
        Some(auth) => Some(auth),
        None => match token {
            RawAuthToken::Bearer(token) => do_user_auth(state, token).await?,
            RawAuthToken::Bot(token) => match token.verify(&state.config().local.keys.bt_key) {
                true => return do_bot_auth(state, token).await,
                false => return Err(Error::Unauthorized),
            },
        },
    };

    match auth {
        Some(auth @ Authorization::Bot { .. }) => Ok(auth),
        Some(auth @ Authorization::User { expires, .. }) if expires > Timestamp::now_utc() => Ok(auth),
        _ => Err(Error::NoSession),
    }
}

pub async fn do_user_auth(state: &ServerState, token: &UserToken) -> Result<Option<Authorization>, Error> {
    let db = state.db.read.get().await?;

    let bytes = &token[..];

    #[rustfmt::skip]
    let row = db.query_opt2(schema::sql! {
        SELECT
            Sessions.UserId  AS @UserId,
            Sessions.Expires AS @Expires,
            Users.Flags      AS @UserFlags
        FROM
            Sessions INNER JOIN Users ON Users.Id = Sessions.UserId
        WHERE
            Sessions.Token = #{&bytes as Sessions::Token}
    }).await?;

    Ok(match row {
        Some(row) => Some({
            let auth = Authorization::User {
                token: *token,
                user_id: row.user_id()?,
                expires: row.expires()?,
                flags: UserFlags::from_bits_truncate(row.user_flags()?),
            };

            state.session_cache.set(auth).await;

            auth
        }),
        None => None,
    })
}

pub async fn do_bot_auth(state: &ServerState, token: &SplitBotToken) -> Result<Authorization, Error> {
    let db = state.db.read.get().await?;

    #[rustfmt::skip]
    let row = db.query_opt2(schema::sql! {
        SELECT Apps.Issued AS @Issued
        FROM Apps WHERE Apps.BotId = #{&token.id as Apps::BotId}
    }).await?;

    let Some(row) = row else {
        return Err(Error::NoSession);
    };

    let issued: u64 = row.issued::<i64>()? as u64;

    unimplemented!()
}
