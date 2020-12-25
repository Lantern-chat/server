use std::{
    cell::Cell,
    time::{Duration, Instant},
};

use crate::util::cmap::CHashMap;

use crate::db::Snowflake;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RateLimitKey {
    pub account: Snowflake,
    pub route: u16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RateLimit {
    pub count: f32,
    pub last: Instant,
}

impl Default for RateLimit {
    fn default() -> Self {
        RateLimit {
            count: 0.0,
            last: Instant::now(),
        }
    }
}

pub struct RateLimitTable {
    pub req_per_sec: f32,
    pub table: CHashMap<RateLimitKey, Cell<RateLimit>>,
}

impl RateLimitTable {
    pub fn new(req_per_sec: f32) -> RateLimitTable {
        RateLimitTable {
            req_per_sec: req_per_sec.max(0.01),
            table: CHashMap::new(128),
        }
    }

    pub async fn req(&self, key: RateLimitKey) -> bool {
        let rll = self.table.get_or_default(&key).await;
        let mut lim = rll.get();

        let now = Instant::now();

        // get the number of decayed requests since the last request
        let decayed = now.duration_since(lim.last).as_millis() as f32 * self.req_per_sec * 0.001;
        // compute the effective number of requests performed
        let eff_count = lim.count - decayed;

        if eff_count < self.req_per_sec {
            // update with new request
            lim.count = eff_count.max(0.0) + 1.0;
            lim.last = now;

            rll.set(lim);

            return true;
        }

        false
    }

    pub async fn cleanup_at(&self, now: Instant) {
        log::trace!("Cleaning old rate-limit entries");

        let one_second_ago = now - Duration::from_secs(1);
        self.table
            .retain(|_, value| value.get().last < one_second_ago)
            .await;
    }
}
