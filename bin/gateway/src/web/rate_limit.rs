use std::hash::{BuildHasher, Hash, Hasher};
use std::time::{Duration, Instant};

use ftl::{
    rate_limit::{Quota, RateLimiter as FtlRateLimiter},
    Route,
};

#[derive(Default)]
pub struct RateLimitTable {
    limiter: FtlRateLimiter<u64, nohash_hasher::BuildNoHashHasher<u64>>,
}

use crate::prelude::*;

impl RateLimitTable {
    pub async fn req(&self, route: &Route<ServerState>) -> bool {
        let key = {
            let mut hasher = route.state.hasher.build_hasher();

            route.real_addr.hash(&mut hasher);

            // split path on /, get alphabetic segments, hash those
            route.path().split('/').take(10).for_each(|segment| {
                if segment.starts_with(|c: char| c.is_ascii_alphabetic()) {
                    segment.hash(&mut hasher);
                }
            });

            hasher.finish()
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
