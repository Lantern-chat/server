use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};

use ftl::{rate_limit::RateLimiter, Route};
use sdk::Snowflake;
use util::cmap::CHashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RateLimitKey {
    pub addr: SocketAddr,
    pub path_hash: u64,
}

#[derive(Default)]
pub struct RateLimitTable {
    pub table: CHashMap<RateLimitKey, RateLimiter>,
}

use crate::ServerState;

impl RateLimitTable {
    pub async fn req(&self, route: &Route<ServerState>) -> bool {
        let key = RateLimitKey {
            addr: route.real_addr,
            path_hash: {
                use std::hash::{BuildHasher, Hash, Hasher};

                let mut hasher = route.state.hasher.build_hasher();

                // split path on /, get alphabetic segments, hash those
                route
                    .path()
                    .split('/')
                    .filter(|s| s.starts_with(|c: char| c.is_alphabetic()))
                    .for_each(|segment| segment.hash(&mut hasher));

                hasher.finish()
            },
        };

        self.table
            .get_mut_or_default(&key)
            .await
            .update(route, route.state.config.web.req_per_sec)
    }

    pub async fn cleanup_at(&self, now: Instant) {
        let one_second_ago = now - Duration::from_secs(1);
        self.table.retain(|_, value| value.last < one_second_ago).await;
    }
}
