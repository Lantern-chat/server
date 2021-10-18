use std::alloc::{GlobalAlloc, Layout};

use server::metric::MEMORY_METRICS;

use util::likely::likely;

#[cfg(not(unix))]
type Global = std::alloc::System;
#[cfg(unix)]
type Global = tikv_jemallocator::Jemalloc;

struct MyAllocator;
unsafe impl GlobalAlloc for MyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = Global {}.alloc(layout);

        if likely(!ret.is_null()) {
            // MEMORY_METRICS.allocs.add(1);
            MEMORY_METRICS.allocated.add(layout.size() as u64);
        }

        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // MEMORY_METRICS.deallocs.add(1);
        MEMORY_METRICS.allocated.sub(layout.size() as u64);

        Global {}.dealloc(ptr, layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ret = Global {}.alloc_zeroed(layout);

        if likely(!ret.is_null()) {
            // MEMORY_METRICS.allocs.add(1);
            MEMORY_METRICS.allocated.add(layout.size() as u64);
        }

        ret
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        MEMORY_METRICS.allocated.sub(layout.size() as u64);

        let ret = Global {}.realloc(ptr, layout, new_size);

        if likely(!ret.is_null()) {
            MEMORY_METRICS.allocated.add(new_size as u64);
        }

        // if unlikely(ret.is_null()) {
        // MEMORY_METRICS.deallocs.add(1);
        // } else {}

        ret
    }
}

#[global_allocator]
static GLOBAL: MyAllocator = MyAllocator;
