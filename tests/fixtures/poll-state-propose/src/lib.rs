// `wit-bindgen` 0.30 emits `unsafe fn` post-return helpers whose bodies call
// unsafe ops directly. Edition 2024's `unsafe_op_in_unsafe_fn` lint defaults
// to deny, so we relax it locally for the generated bindings. The wit-bindgen
// upstream addresses this in newer releases; we pin 0.30 per the plan.
#![allow(unsafe_op_in_unsafe_fn)]
#![no_std]
#![no_main]
extern crate alloc;

use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};

// Minimal bump allocator. State-propose runs a fresh instance per
// intent; the kernel discards the store after each call so we never
// actually need to free. A bump allocator is the smallest thing that
// satisfies the `extern crate alloc` requirement without pulling in
// float-Display paths from any of the ecosystem crates.
//
// Memory cap is enforced kernel-side via Wasmtime StoreLimits; this
// allocator just hands out aligned slots from a fixed-size byte array
// inside the wasm linear memory.
const HEAP_SIZE: usize = 64 * 1024;

#[repr(C, align(16))]
struct BumpHeap {
    bytes: core::cell::UnsafeCell<[u8; HEAP_SIZE]>,
    next: core::sync::atomic::AtomicUsize,
}

unsafe impl Sync for BumpHeap {}

#[allow(unsafe_code)]
static HEAP: BumpHeap = BumpHeap {
    bytes: core::cell::UnsafeCell::new([0; HEAP_SIZE]),
    next: core::sync::atomic::AtomicUsize::new(0),
};

struct BumpAlloc;

#[allow(unsafe_code)]
unsafe impl GlobalAlloc for BumpAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        use core::sync::atomic::Ordering;
        let align = layout.align().max(1);
        let size = layout.size();
        let base = HEAP.bytes.get() as usize;
        let mut cur = HEAP.next.load(Ordering::Relaxed);
        loop {
            let aligned = (base + cur + (align - 1)) & !(align - 1);
            let new_cur = (aligned - base).saturating_add(size);
            if new_cur > HEAP_SIZE {
                return core::ptr::null_mut();
            }
            match HEAP.next.compare_exchange_weak(
                cur,
                new_cur,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return aligned as *mut u8,
                Err(observed) => cur = observed,
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator: never frees. State-propose discards the store
        // after each intent, so the cumulative HEAP_SIZE budget bounds
        // the per-call allocation, not the lifetime allocation.
    }
}

#[global_allocator]
static GLOBAL: BumpAlloc = BumpAlloc;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // wasm32 has no native trap-from-rust; loop {} translates to a
    // wasm `unreachable` after the optimizer notices it never returns.
    loop {}
}

wit_bindgen::generate!({
    world: "state-propose",
});

struct Component;

export!(Component);

// T0 placeholder. T1 lands the byte-offset decoder; T2 lands the intent
// vocabulary (CreatePoll / Vote / EndPoll). Until then propose
// unconditionally errors so any test that accidentally exercises this
// fixture pre-T2 fails loudly rather than emitting an empty payload
// that downstream apply (also a placeholder) would mishandle.
impl Guest for Component {
    fn propose(
        _prior_state: Vec<u8>,
        _intent: Vec<u8>,
    ) -> Result<Vec<u8>, alloc::string::String> {
        Err(alloc::string::String::from("unimplemented"))
    }
}
