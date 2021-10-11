use std::{
    ops::Deref,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

#[derive(Default)]
#[repr(transparent)]
pub struct Counter(AtomicU64);

#[derive(Default)]
#[repr(transparent)]
pub struct PaddedCounter(CachePadded<Counter>);

#[repr(transparent)]
pub struct LatencyHistogram([u64; 1000]);

impl Default for LatencyHistogram {
    fn default() -> Self {
        LatencyHistogram::new()
    }
}

use arc_swap::ArcSwap;
use cache_padded::CachePadded;
use ftl::StatusCode;

pub static MEMORY_METRICS: MemoryMetrics = MemoryMetrics {
    // allocs: PaddedCounter::new(),
    // deallocs: PaddedCounter::new(),
    allocated: PaddedCounter::new(),
};

lazy_static::lazy_static! {
    pub static ref API_METRICS: ArcSwap<ApiMetrics> = ArcSwap::from_pointee(ApiMetrics::default());
}

pub struct MemoryMetrics {
    // pub allocs: PaddedCounter,
    // pub deallocs: PaddedCounter,
    pub allocated: PaddedCounter,
}

#[derive(Default)]
pub struct ApiMetrics {
    pub reqs: Counter,
    pub errs: Counter,
    pub events: Counter,
    pub upload: Counter,
    pub histogram: LatencyHistogram,
}

impl ApiMetrics {
    pub fn add_req(&self, status: StatusCode, duration: Duration) {
        self.reqs.add(1);

        let status = status.as_u16();
        if util::likely::unlikely(!(100 <= status && status < 500)) {
            self.errs.add(1);
        }

        self.histogram.add(duration.as_millis() as usize);
    }

    // compute percentiles and reset the histogram and req counter
    pub fn percentiles(&self) -> (u64, [u16; 3]) {
        let histogram = self.histogram.as_ref();

        let count = self.reqs.get();
        let countf = count as f64;

        let targets: [f64; 3] = [countf * 0.5, countf * 0.95, countf * 0.99];
        let mut percentiles = [u16::MAX; 3];

        let mut sumf = 0.0;
        let mut i = 0;

        for (idx, val) in histogram.iter().enumerate() {
            sumf += val.get() as f64;

            if sumf >= targets[i] {
                percentiles[i] = idx as u16;
                i += 1;

                if i == 3 {
                    break;
                }
            }
        }

        (count, percentiles)
    }
}

impl Counter {
    pub const fn new() -> Self {
        Counter(AtomicU64::new(0))
    }

    #[inline]
    pub fn add(&self, count: u64) {
        self.0.fetch_add(count, Ordering::Relaxed);
    }

    pub fn sub(&self, count: u64) {
        self.0.fetch_sub(count, Ordering::Relaxed);
    }

    #[inline]
    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn reset(&self) -> u64 {
        self.0.swap(0, Ordering::Relaxed)
    }
}

impl Deref for PaddedCounter {
    type Target = Counter;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PaddedCounter {
    pub const fn new() -> Self {
        PaddedCounter(CachePadded::new(Counter::new()))
    }
}

impl AsRef<[Counter]> for LatencyHistogram {
    #[inline(always)]
    fn as_ref(&self) -> &[Counter] {
        unsafe { std::mem::transmute::<&[u64], &[Counter]>(&self.0.as_ref()) }
    }
}

impl LatencyHistogram {
    pub const fn new() -> Self {
        LatencyHistogram([0u64; 1000])
    }

    #[inline]
    pub fn add(&self, ms: usize) {
        unsafe {
            std::mem::transmute::<&u64, &Counter>(self.0.get_unchecked(ms.min(999))).add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Counter;

    #[test]
    fn test_atomic_counter() {
        use std::mem::{align_of, size_of};

        assert_eq!(size_of::<u64>(), size_of::<Counter>());
        assert_eq!(align_of::<u64>(), align_of::<Counter>());
    }
}
