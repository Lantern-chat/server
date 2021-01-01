use std::hash::{BuildHasher, Hash, Hasher};

use ahash::{AHasher, RandomState};
use hashbrown::HashMap;
use tokio::sync::RwLock;

use crate::{
    db::{schema::user, Snowflake},
    util::cmap::CHashMap,
};

pub struct ClientSubscriptions {
    pub parties: CHashMap<Snowflake, PartySubscriptions, RandomState>,
}

pub struct PartySubscriptions {
    pub clients: Vec<()>,

    /// HashSet with 16-bit key to avoid collisions, but the hash
    /// used is formed from the full 128-bit key `(Snowflake, Snowflake)`
    pub channel_users: RwLock<HashMap<u16, (), RandomState>>,
}

// TODO: Refresh the channel_users set on party updates (user left/joined/changed roles)
impl PartySubscriptions {
    #[inline]
    pub async fn channel_has_user(&self, channel_id: Snowflake, user_id: Snowflake) -> bool {
        let full_key: u128 = unsafe { std::mem::transmute((channel_id, user_id)) };

        // take the lower 8 bits of each snowflake to use as the key, purely to avoid collisions
        let low_channel = (channel_id.raw_timestamp() as u16) << 8;
        let low_user = (user_id.raw_timestamp() as u16) & 0xFF;
        let short_key = low_channel | low_user;

        let channel_users = self.channel_users.read().await;

        let mut hasher = channel_users.hasher().build_hasher();
        hasher.write_u128(full_key); // aHash processes 128-bits at once

        channel_users
            .raw_entry()
            .from_key_hashed_nocheck(hasher.finish(), &short_key)
            .is_some()
    }
}
