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
pub struct RateLimiter {
    pub count: f32,
    pub last: Instant,
}

impl Default for RateLimiter {
    fn default() -> Self {
        RateLimiter {
            count: 0.0,
            last: Instant::now(),
        }
    }
}

impl RateLimiter {
    #[inline]
    pub fn update(&mut self, req_per_sec: f32) -> bool {
        let now = Instant::now();

        // get the number of decayed requests since the last request
        let decayed = now.duration_since(self.last).as_millis() as f32 * req_per_sec * 0.001;
        // compute the effective number of requests performed
        let eff_count = self.count - decayed;

        if eff_count < req_per_sec {
            // update with new request
            self.count = eff_count.max(0.0) + 1.0;
            self.last = now;

            return true;
        }

        false
    }
}

pub struct RateLimitTable {
    pub req_per_sec: f32,
    pub table: CHashMap<RateLimitKey, RateLimiter>,
}

impl RateLimitTable {
    pub fn new(req_per_sec: f32) -> RateLimitTable {
        RateLimitTable {
            req_per_sec: req_per_sec.max(0.01),
            table: CHashMap::new(128),
        }
    }

    pub async fn req(&self, key: RateLimitKey) -> bool {
        self.table
            .get_mut_or_default(&key)
            .await
            .update(self.req_per_sec)
    }

    pub async fn cleanup_at(&self, now: Instant) {
        log::trace!("Cleaning old rate-limit entries");

        let one_second_ago = now - Duration::from_secs(1);
        self.table
            .retain(|_, value| value.last < one_second_ago)
            .await;
    }
}
