use std::hash::{BuildHasher, Hash, Hasher};
use std::time::{Duration, Instant};

use ftl::{
    rate_limit::{Quota, RateLimiter as FtlRateLimiter},
    Route,
};

#[derive(Default)]
pub struct RateLimitTable {
    limiter: FtlRateLimiter<u64, NoHasherBuilder>,
}

use crate::ServerState;

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

/// A very specific hasher to use the u64 key as the hash itself,
/// since it's already a hash.
#[derive(Debug, Clone, Copy)]
struct NoHasher([u8; 8]);
#[derive(Debug, Default)]
struct NoHasherBuilder;

impl Hasher for NoHasher {
    #[inline(always)]
    fn finish(&self) -> u64 {
        u64::from_ne_bytes(self.0)
    }

    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) {
        self.0.copy_from_slice(bytes);
    }
}

impl BuildHasher for NoHasherBuilder {
    type Hasher = NoHasher;

    #[inline(always)]
    fn build_hasher(&self) -> Self::Hasher {
        NoHasher([0; 8])
    }
}
