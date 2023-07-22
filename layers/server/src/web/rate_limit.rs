use std::time::{Duration, Instant};

use ftl::{
    rate_limit::{Quota, RateLimiter as FtlRateLimiter},
    Route,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RateLimitKey {
    pub addr_hash: u64,
    pub path_hash: u64,
}

#[derive(Default)]
pub struct RateLimitTable {
    pub limiter: FtlRateLimiter<RateLimitKey>,
}

use crate::ServerState;

impl RateLimitTable {
    pub async fn req(&self, route: &Route<ServerState>) -> bool {
        let key = RateLimitKey {
            addr_hash: route.state.hasher.hash_one(route.real_addr),
            path_hash: {
                use std::hash::{BuildHasher, Hash, Hasher};

                let mut hasher = route.state.hasher.build_hasher();

                // split path on /, get alphabetic segments, hash those
                route.path().split('/').for_each(|segment| {
                    if segment.starts_with(|c: char| c.is_ascii_alphabetic()) {
                        segment.hash(&mut hasher);
                    }
                });

                hasher.finish()
            },
        };

        // TODO: Compute this during hash lookup?
        let quota = Quota::new(Duration::from_millis(20), 10.try_into().unwrap());
        self.limiter.req(key, quota, route.start).await.is_ok()
    }

    pub async fn cleanup_at(&self, now: Instant) {
        let one_second_ago = now.checked_sub(Duration::from_secs(1)).expect("Failed to subtract one second");
        self.limiter.clean(one_second_ago).await;
    }
}
