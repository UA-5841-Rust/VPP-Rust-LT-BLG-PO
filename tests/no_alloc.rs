//! Dedicated integration-test process: the only test counts hot-path allocations.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct CountingAllocator;
static ENABLED: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);

// SAFETY: all allocation operations delegate unchanged to System; atomics only
// observe calls and do not alter allocation sizes, alignment, or ownership.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ENABLED.load(Ordering::Relaxed) {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: GlobalAlloc supplies a valid layout, forwarded unchanged.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: caller guarantees a live allocation with this layout; the
        // pointer came from System through the allocator above.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn classification_allocates_nothing() {
    let mut bytes = [0u8; 42];
    bytes[12] = 8;
    bytes[14] = 0x45;
    bytes[17] = 28;
    bytes[23] = 17;
    bytes[39] = 8;
    ALLOCS.store(0, Ordering::Relaxed);
    ENABLED.store(true, Ordering::Relaxed);
    for _ in 0..1000 {
        for len in 0..=bytes.len() {
            // SAFETY: len is bounded by this live, immutable stack allocation.
            std::hint::black_box(unsafe {
                network_parser::ffi::packet_classify(bytes.as_ptr(), len)
            });
        }
    }
    ENABLED.store(false, Ordering::Relaxed);
    assert_eq!(ALLOCS.load(Ordering::Relaxed), 0);
}
