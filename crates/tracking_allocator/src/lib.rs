#![no_std]
#![allow(clippy::let_and_return, unused_imports)]

extern crate alloc;

use core::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use crossbeam_utils::CachePadded;

use alloc::alloc::{GlobalAlloc, Layout};

#[cfg(feature = "enable")]
#[repr(C)]
pub struct TrackingAllocator<A> {
    allocator: A,
    allocated: CachePadded<AtomicUsize>,
}

#[cfg(not(feature = "enable"))]
#[repr(transparent)]
pub struct TrackingAllocator<A> {
    allocator: A,
}

impl<A> TrackingAllocator<A> {
    pub const fn new(allocator: A) -> Self {
        TrackingAllocator {
            allocator,

            #[cfg(feature = "enable")]
            allocated: CachePadded::new(AtomicUsize::new(0)),
        }
    }

    #[inline]
    pub fn allocated(&self) -> usize {
        #[cfg(feature = "enable")]
        return self.allocated.load(Relaxed);

        #[cfg(not(feature = "enable"))]
        return 0;
    }
}

unsafe impl<A: GlobalAlloc> GlobalAlloc for TrackingAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = self.allocator.alloc(layout);

        #[cfg(feature = "enable")]
        if likely(!ret.is_null()) {
            self.allocated.fetch_add(layout.size(), Relaxed);
        }

        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        #[cfg(feature = "enable")]
        self.allocated.fetch_sub(layout.size(), Relaxed);

        self.allocator.dealloc(ptr, layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ret = self.allocator.alloc_zeroed(layout);

        #[cfg(feature = "enable")]
        if likely(!ret.is_null()) {
            self.allocated.fetch_add(layout.size(), Relaxed);
        }

        ret
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        #[cfg(feature = "enable")]
        self.allocated.fetch_sub(layout.size(), Relaxed);

        let ret = self.allocator.realloc(ptr, layout, new_size);

        #[cfg(feature = "enable")]
        if likely(!ret.is_null()) {
            self.allocated.fetch_add(new_size, Relaxed);
        }

        ret
    }
}

#[rustfmt::skip]
#[inline(always)]
pub fn likely(b: bool) -> bool {
    #[inline]
    #[cold]
    fn cold() {}

    if !b { cold() } b
}
