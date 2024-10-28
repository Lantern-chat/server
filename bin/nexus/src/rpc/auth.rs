use crate::prelude::*;

use schema::auth::{ArchivedRawAuthToken, RawAuthToken, SplitBotToken, UserToken};

pub trait AuthTokenExt {
    fn random_bearer() -> Self;
}

impl AuthTokenExt for RawAuthToken {
    fn random_bearer() -> Self {
        RawAuthToken::bearer(util::rng::crypto_thread_rng())
    }
}

use sdk::models::{Timestamp, UserFlags};

pub async fn do_auth(state: ServerState, token: &Archived<RawAuthToken>) -> Result<Authorization, Error> {
    let auth = match token {
        ArchivedRawAuthToken::Bearer(token) => do_user_auth(state, token).await?,
        ArchivedRawAuthToken::Bot(token) => do_bot_auth(state, token).await?,
    };

    match auth {
        Some(auth @ Authorization::Bot { .. }) => Ok(auth),
        Some(auth @ Authorization::User { expires, .. }) if expires > Timestamp::now_utc() => Ok(auth),
        _ => Err(Error::NoSession),
    }
}

pub async fn do_user_auth(
    state: ServerState,
    token: &Archived<UserToken>,
) -> Result<Option<Authorization>, Error> {
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
        Some(row) => Some(Authorization::User {
            token: *token,
            user_id: row.user_id()?,
            expires: row.expires()?,
            flags: UserFlags::from_bits_truncate(row.user_flags()?),
        }),
        None => None,
    })
}

pub async fn do_bot_auth(
    state: ServerState,
    token: &Archived<SplitBotToken>,
) -> Result<Option<Authorization>, Error> {
    use std::time::Duration;

    let db = state.db.read.get().await?;

    #[rustfmt::skip]
    let row = db.query_opt2(schema::sql! {
        SELECT Apps.Issued AS @Issued FROM Apps WHERE Apps.BotId = #{&token.id as Apps::BotId}
    }).await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let issued = row.issued::<i64>()? as u64;

    // Once the token has passed the HMAC check, we know it's genuine, but
    // it could have still been revoked, so we need to check the issued time
    // to make sure it's still valid
    if token.issued != issued {
        return Ok(None); // interpreted as NoSession by the RPC client
    }

    Ok(Some(Authorization::Bot {
        bot_id: token.id.into(),
        issued: Timestamp::UNIX_EPOCH + Duration::from_secs(issued),
    }))
}
