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
    pub async fn penalize(&self, state: &ServerState, addr: std::net::SocketAddr, penalty: u64) {
        // convert penalty to nanoseconds and apply it to the rate limiter
        self.limiter.penalize(&state.hasher.hash_one(addr), penalty * 1_000_000).await;
    }

    pub async fn req(&self, route: &Route<ServerState>) -> Result<(), Duration> {
        let (ip_key, path_key) = {
            let mut hasher = route.state.hasher.build_hasher();

            route.real_addr.hash(&mut hasher);

            let ip_key = hasher.finish();

            // split path on /, get alphabetic segments, hash those
            route.path().split('/').take(10).for_each(|segment| {
                if segment.starts_with(|c: char| c.is_ascii_alphabetic()) {
                    segment.hash(&mut hasher);
                }
            });

            (ip_key, hasher.finish())
        };

        // TODO: Compute this during hash lookup
        let quota = Quota::new(Duration::from_millis(20), 10.try_into().unwrap());

        // Per-IP rate limit 1000 requests per second with a burst of 100
        let global_quota = Quota::new(Duration::from_millis(1), 100.try_into().unwrap());

        let res = tokio::join! {
            self.limiter.req(ip_key, global_quota, route.start),
            self.limiter.req(path_key, quota, route.start),
        };

        match res {
            (Ok(_), Ok(_)) => Ok(()),
            (Err(a), Err(b)) => Err(a.max(b).as_duration()),
            (Err(e), _) | (_, Err(e)) => Err(e.as_duration()),
        }
    }

    pub async fn cleanup_at(&self, now: Instant) {
        let one_second_ago = now.checked_sub(Duration::from_secs(1)).expect("Failed to subtract one second");
        self.limiter.clean(one_second_ago).await;
    }
}
