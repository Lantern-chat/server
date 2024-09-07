use timestamp::Timestamp;

use crate::prelude::*;

/// converts a timestamp to decaseconds elapsed since `now`, randomized
pub fn approximate_relative_time(
    state: &ServerState,
    user_id: UserId,
    ts: Option<Timestamp>,
    now: Option<Timestamp>,
) -> Option<u64> {
    use core::hash::BuildHasher;

    let ts = ts?;

    let elapsed = now.unwrap_or_else(Timestamp::now_utc).duration_since(ts);

    // convert to decaseconds
    let elapsed = elapsed.whole_seconds() / 10;

    if elapsed <= 0 {
        return Some(0);
    }

    // 0..1 based on ts and user_id
    let hash = state.hasher.hash_one((user_id, ts)) as f64 * (1.0 / u64::MAX as f64);
    // -1..1
    let factor = hash.mul_add(2.0, -1.0);
    // scale by relative_time_random_factor
    let random = state.config().shared.relative_time_random_factor as f64 * factor;
    // offset elapsed by a random amount proportional to elapsed itself
    let randomized_elapsed = (elapsed as f64).mul_add(random, elapsed as f64);
    // convert back to u64
    Some(randomized_elapsed.max(0.0) as u64)
}
