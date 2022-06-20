#[cfg(not(unix))]
type Global = std::alloc::System;
#[cfg(unix)]
type Global = tikv_jemallocator::Jemalloc;

#[cfg(feature = "memory_metrics")]
#[global_allocator]
static GLOBAL: allocator::MyAllocator = allocator::MyAllocator;

#[cfg(not(feature = "memory_metrics"))]
#[global_allocator]
static GLOBAL: Global = Global {};

#[cfg(feature = "memory_metrics")]
mod allocator {
    use super::Global;

    use server::metrics::MEMORY_METRICS;
    use std::alloc::{GlobalAlloc, Layout};
    use util::likely::likely;

    pub struct MyAllocator;
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
}
