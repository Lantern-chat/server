use sdk::models::{aliases::*, Timestamp, UserFlags};

use schema::auth::{RawAuthToken, UserToken};

use rpc::auth::Authorization;

use crate::prelude::*;

#[derive(Debug, Clone, Copy)]
struct PartialUserAuthorization {
    user_id: UserId,
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
    users: scc::HashIndex<UserToken, PartialUserAuthorization, sdk::FxRandomState2>,
    invalid: scc::HashSet<RawAuthToken, sdk::FxRandomState2>,
    bots: scc::HashIndex<UserId, PartialBotAuthorization, sdk::FxRandomState2>,
}

impl AuthCache {
    pub fn get(&self, token: &RawAuthToken, state: &ServerState) -> Result<Option<Authorization>, Error> {
        if self.invalid.contains(token) {
            return Err(Error::Unauthorized);
        }

        Ok(match token {
            RawAuthToken::Bearer(token) => self.users.peek_with(token, |_, partial| Authorization::User {
                token: *token,
                user_id: partial.user_id,
                expires: partial.expires,
                flags: partial.flags,
            }),
            RawAuthToken::Bot(token) if token.verify(&state.config().local.keys.bt_key) => {
                self.bots.peek_with(&token.id, |_, partial| Authorization::Bot {
                    bot_id: token.id,
                    issued: partial.issued,
                })
            }
            _ => return Err(Error::Unauthorized),
        })
    }

    pub async fn set_invalid(&self, token: RawAuthToken) {
        _ = tokio::join! {
            self.invalid.insert_async(token),
            async {
                #[allow(clippy::single_match)] // until fixed by adding bot removal
                match token {
                    RawAuthToken::Bearer(token) => _ = self.users.remove_async(&token).await,
                    _ => {} // TODO: Implement bot removal
                }
            },
        };
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

    // pub async fn clear_user(&self, user_id: UserId) {
    //     self.map.retain_async(|_, part| part.user_id != user_id).await;
    // }
}
