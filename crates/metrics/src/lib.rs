#![no_std]

use core::{
    ops::Deref,
    sync::atomic::{AtomicU64, Ordering::Relaxed},
};

use crossbeam_utils::CachePadded;

#[derive(Default)]
#[repr(transparent)]
pub struct Counter(AtomicU64);

#[derive(Default)]
#[repr(transparent)]
pub struct PaddedCounter(CachePadded<Counter>);

pub struct LatencyHistogram {
    /// 1024 buckets for latency histogram. `LatencyHistogram` has to be 128-bit aligned
    /// on x86 and aarch64 anyway so fill the rest of the space with counters,
    /// even if it'll look weird to have up to 1024ms latency. On platforms
    /// with smaller alignment requirements, whatever.
    h: [Counter; 1024],
    c: PaddedCounter,
}

impl Default for LatencyHistogram {
    #[inline]
    fn default() -> Self {
        LatencyHistogram::new()
    }
}

impl Counter {
    #[inline]
    pub const fn new() -> Self {
        Counter(AtomicU64::new(0))
    }

    #[inline(always)]
    pub fn add(&self, count: u64) -> u64 {
        self.0.fetch_add(count, Relaxed)
    }

    #[inline(always)]
    pub fn sub(&self, count: u64) -> u64 {
        self.0.fetch_sub(count, Relaxed)
    }

    #[inline(always)]
    pub fn get(&self) -> u64 {
        self.0.load(Relaxed)
    }

    #[inline(always)]
    pub fn reset(&self) -> u64 {
        self.0.swap(0, Relaxed)
    }
}

impl Deref for PaddedCounter {
    type Target = Counter;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PaddedCounter {
    #[inline]
    pub const fn new() -> Self {
        PaddedCounter(CachePadded::new(Counter::new()))
    }
}

impl LatencyHistogram {
    #[inline]
    pub const fn new() -> Self {
        LatencyHistogram {
            h: [const { Counter::new() }; 1024],
            c: PaddedCounter::new(),
        }
    }

    #[inline]
    pub fn count(&self) -> u64 {
        self.c.get()
    }

    #[inline]
    pub fn add(&self, ms: usize) {
        self.c.add(1);
        self.h[ms.min(1023)].add(1);
    }

    // compute latency percentiles `[0.5, 0.95, 0.99]`
    pub fn percentiles(&self) -> (u64, [u16; 3]) {
        let count = self.count();
        let histogram = &self.h;

        let countf = count as f64;

        let targets: [f64; 3] = [countf * 0.5, countf * 0.95, countf * 0.99];
        let mut percentiles = [u16::MAX; 3];

        let mut sum = 0.0;
        let mut i = 0;

        'outer: for (idx, val) in histogram.iter().enumerate() {
            sum += val.get() as f64;

            while sum >= targets[i] {
                percentiles[i] = idx as u16;

                i += 1;

                if i == 3 {
                    break 'outer;
                }
            }
        }

        (count, percentiles)
    }
}
