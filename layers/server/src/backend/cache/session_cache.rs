use std::time::SystemTime;

use sdk::models::UserFlags;

use schema::{auth::RawAuthToken, Snowflake};

use crate::backend::api::auth::Authorization;

#[derive(Debug, Clone, Copy)]
struct PartialAuthorization {
    user_id: Snowflake,
    expires: SystemTime,
    flags: UserFlags,
}

#[derive(Default)]
pub struct SessionCache {
    map: scc::HashIndex<RawAuthToken, PartialAuthorization, ahash::RandomState>,
}

impl SessionCache {
    pub fn get(&self, token: &RawAuthToken) -> Option<Authorization> {
        self.map.read(token, |_, part| Authorization {
            token: *token,
            user_id: part.user_id,
            expires: part.expires,
            flags: part.flags,
        })
    }

    pub async fn set(&self, auth: Authorization) {
        let part = PartialAuthorization {
            user_id: auth.user_id,
            expires: auth.expires,
            flags: auth.flags,
        };

        if let Ok(_) = self.map.insert_async(auth.token, part).await {
            log::trace!("Cached auth {}: {}", auth.user_id, auth.token);
        }
    }

    pub async fn cleanup(&self, now: SystemTime) {
        self.map.retain_async(|_, part| part.expires < now).await;
    }

    // pub async fn clear_user(&self, user_id: Snowflake) {
    //     self.map.retain_async(|_, part| part.user_id != user_id).await;
    // }
}
