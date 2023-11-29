use sdk::models::{Timestamp, UserFlags};

use schema::{
    auth::{RawAuthToken, UserToken},
    Snowflake,
};

use crate::backend::api::auth::Authorization;

#[derive(Debug, Clone, Copy)]
struct PartialUserAuthorization {
    user_id: Snowflake,
    expires: Timestamp,
    flags: UserFlags,
}

#[derive(Debug, Clone, Copy)]
struct PartialBotAuthorization {
    issued: Timestamp,
    _flags: (),
}

#[derive(Default)]
pub struct AuthCache {
    users: scc::HashIndex<UserToken, PartialUserAuthorization, ahash::RandomState>,
    bots: scc::HashIndex<Snowflake, PartialBotAuthorization, ahash::RandomState>,
}

impl AuthCache {
    pub fn get(&self, token: &RawAuthToken) -> Option<Authorization> {
        match token {
            RawAuthToken::Bearer(token) => self.users.peek_with(token, |_, partial| Authorization::User {
                token: *token,
                user_id: partial.user_id,
                expires: partial.expires,
                flags: partial.flags,
            }),
            RawAuthToken::Bot(token) => self.bots.peek_with(&token.id, |_, partial| Authorization::Bot {
                bot_id: token.id,
                issued: partial.issued,
            }),
        }
    }

    pub async fn set(&self, auth: Authorization) {
        match auth {
            Authorization::User {
                token,
                user_id,
                expires,
                flags,
            } => {
                let partial = PartialUserAuthorization {
                    user_id,
                    expires,
                    flags,
                };

                if let Ok(_) = self.users.insert_async(token, partial).await {
                    log::trace!("Cached auth {}: {:?}", user_id, token);
                }
            }
            Authorization::Bot { bot_id, issued } => {
                let partial = PartialBotAuthorization { issued, _flags: () };

                if let Ok(_) = self.bots.insert_async(bot_id, partial).await {
                    log::trace!("Cached bot auth: {}", bot_id);
                }
            }
        }
    }

    pub async fn cleanup(&self, now: Timestamp) {
        self.users.retain_async(|_, part| part.expires < now).await;
    }

    // pub async fn clear_user(&self, user_id: Snowflake) {
    //     self.map.retain_async(|_, part| part.user_id != user_id).await;
    // }
}
