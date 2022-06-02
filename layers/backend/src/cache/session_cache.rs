use std::time::SystemTime;

use sdk::models::UserFlags;
use util::cmap::CHashMap;

use schema::{auth::RawAuthToken, Snowflake};

use crate::api::auth::Authorization;

#[derive(Debug, Clone, Copy)]
struct PartialAuthorization {
    user_id: Snowflake,
    expires: SystemTime,
    flags: UserFlags,
}

#[derive(Default)]
pub struct SessionCache {
    map: CHashMap<RawAuthToken, PartialAuthorization>,
}

impl SessionCache {
    pub async fn get(&self, token: &RawAuthToken) -> Option<Authorization> {
        match self.map.get(token).await {
            None => None,
            Some(part) => Some(Authorization {
                token: *token,
                user_id: part.user_id,
                expires: part.expires,
                flags: part.flags,
            }),
        }
    }

    pub async fn set(&self, auth: Authorization) {
        self.map
            .insert(
                auth.token,
                PartialAuthorization {
                    user_id: auth.user_id,
                    expires: auth.expires,
                    flags: auth.flags,
                },
            )
            .await;

        log::trace!("Cached auth {}: {}", auth.user_id, auth.token);
    }

    pub async fn cleanup(&self, now: SystemTime) {
        self.map.retain(|_, part| part.expires < now).await;
    }

    pub async fn clear_user(&self, user_id: Snowflake) {
        self.map.retain(|_, part| part.user_id != user_id).await;
    }
}
