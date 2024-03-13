#[cfg(not(all(unix, any(target_arch = "x86", target_arch = "x86_64"))))]
type Global = std::alloc::System;
#[cfg(all(unix, any(target_arch = "x86", target_arch = "x86_64")))]
type Global = tikv_jemallocator::Jemalloc;

use tracking_allocator::TrackingAllocator;

//#[cfg(feature = "memory_metrics")]
#[global_allocator]
pub static GLOBAL: TrackingAllocator<Global> = TrackingAllocator::new(Global {});

//#[cfg(not(feature = "memory_metrics"))]
//#[global_allocator]
//pub static GLOBAL: Global = Global {};
