use std::hash::{BuildHasher, Hash, Hasher};

use ahash::{AHasher, RandomState};
use hashbrown::HashMap;
use warp::path::full;

use crate::db::{schema::user, Snowflake};

pub struct ClientSubscriptions {
    parties: HashMap<Snowflake, PartySubscriptions, RandomState>,
}

pub struct PartySubscriptions {
    clients: Vec<()>,

    /// HashSet with 16-bit key to avoid collisions, but the hash
    /// used is formed from the full 128-bit key `(Snowflake, Snowflake)`
    channel_users: HashMap<u16, (), RandomState>,
}

impl ClientSubscriptions {
    #[inline]
    pub fn get_party(&self, party_id: Snowflake) -> Option<&PartySubscriptions> {
        self.parties.get(&party_id)
    }
}

// TODO: Refresh the channel_users set on party updates (user left/joined/changed roles)
impl PartySubscriptions {
    #[inline]
    pub fn channel_has_user(&self, channel_id: Snowflake, user_id: Snowflake) -> bool {
        let full_key: u128 = unsafe { std::mem::transmute((channel_id, user_id)) };

        // take the lower 8 bits of each snowflake to use as the key, purely to avoid collisions
        let low_channel = (channel_id.raw_timestamp() as u16) << 8;
        let low_user = (user_id.raw_timestamp() as u16) & 0xFF;
        let short_key = low_channel | low_user;

        let mut hasher = self.channel_users.hasher().build_hasher();
        hasher.write_u128(full_key); // aHash processes 128-bits at once

        self.channel_users
            .raw_entry()
            .from_key_hashed_nocheck(hasher.finish(), &short_key)
            .is_some()
    }
}
