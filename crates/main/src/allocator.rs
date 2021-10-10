use std::alloc::{GlobalAlloc, Layout, System};

struct MyAllocator;

use server::metric::MEMORY_METRICS;

use util::likely::likely;

unsafe impl GlobalAlloc for MyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = System.alloc(layout);

        if likely(!ret.is_null()) {
            // MEMORY_METRICS.allocs.add(1);
            MEMORY_METRICS.allocated.add(layout.size() as u64);
        }

        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // MEMORY_METRICS.deallocs.add(1);
        MEMORY_METRICS.allocated.sub(layout.size() as u64);

        System.dealloc(ptr, layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ret = System.alloc_zeroed(layout);

        if likely(!ret.is_null()) {
            // MEMORY_METRICS.allocs.add(1);
            MEMORY_METRICS.allocated.add(layout.size() as u64);
        }

        ret
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        MEMORY_METRICS.allocated.sub(layout.size() as u64);

        let ret = System.realloc(ptr, layout, new_size);

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
