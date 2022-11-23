use std::fmt;
use std::str::FromStr;
use std::time::SystemTime;

use schema::auth::{AuthToken, RawAuthToken, SplitBotToken};

pub trait AuthTokenExt {
    fn random_bearer() -> Self;
}

impl AuthTokenExt for RawAuthToken {
    fn random_bearer() -> Self {
        RawAuthToken::bearer(util::rng::crypto_thread_rng())
    }

    //fn random_bot() -> Self {
    //    RawAuthToken::bot(util::rng::crypto_thread_rng())
    //}
}

use schema::Snowflake;
use sdk::models::UserFlags;

use crate::ServerState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Authorization {
    pub token: RawAuthToken,
    pub user_id: Snowflake,
    pub expires: SystemTime,
    pub flags: UserFlags,
}

impl Authorization {
    pub fn is_bot(&self) -> bool {
        matches!(self.token, RawAuthToken::Bot(_))
    }
}

use crate::Error;

pub async fn do_auth(state: &ServerState, token: RawAuthToken) -> Result<Authorization, Error> {
    let auth = match state.session_cache.get(&token).await {
        Some(auth) => Some(auth),
        None => match token {
            RawAuthToken::Bearer(bytes) => do_user_auth(state, &bytes, token).await?,
            RawAuthToken::Bot(token) => match token.verify(&state.config.keys.bt_key) {
                false => return Err(Error::Unauthorized),
                true => do_bot_auth(state, token).await?,
            },
        },
    };

    match auth {
        Some(auth) if auth.expires > SystemTime::now() => Ok(auth),
        _ => Err(Error::NoSession),
    }
}

pub async fn do_user_auth(
    state: &ServerState,
    bytes: &[u8],
    token: RawAuthToken,
) -> Result<Option<Authorization>, Error> {
    let db = state.db.read.get().await?;

    let row = db
        .query_opt_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                // NOTE: User-deletion process will delete session tokens, too.
                Query::select()
                    .cols(&[Sessions::UserId, Sessions::Expires])
                    .col(Users::Flags)
                    .from(Sessions::inner_join_table::<Users>().on(Users::Id.equals(Sessions::UserId)))
                    .and_where(Sessions::Token.equals(Var::of(Sessions::Token)))
            },
            &[&bytes],
        )
        .await?;

    Ok(match row {
        Some(row) => Some({
            let auth = Authorization {
                token,
                user_id: row.try_get(0)?,
                expires: row.try_get(1)?,
                flags: UserFlags::from_bits_truncate(row.try_get(2)?),
            };

            state.session_cache.set(auth).await;

            auth
        }),
        None => None,
    })
}

pub async fn do_bot_auth(state: &ServerState, token: SplitBotToken) -> Result<Option<Authorization>, Error> {
    unimplemented!()
}
