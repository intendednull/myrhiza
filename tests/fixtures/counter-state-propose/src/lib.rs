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

// Counter v1 intent vocabulary (app-internal contract between
// counter-interaction and counter-state-propose — opaque to the kernel):
//
//   intent[0]    = 0x00  // Increment discriminator
//   intent[1..9] = i64 BE delta
//
// Propose validates the intent and emits the 8-byte BE delta as the
// event payload, matching counter-state-apply's non-genesis payload
// shape (8-byte BE i64 increment applied to the running counter).
//
// prior_state is not inspected here — counter-state-apply checks for
// overflow when applying the event. The kernel re-runs state-apply in
// dry-run (pre-check) after propose, so any overflow that slipped
// through would surface there before the event is signed.

impl Guest for Component {
    fn propose(prior_state: Vec<u8>, intent: Vec<u8>) -> Result<Vec<u8>, alloc::string::String> {
        // Suppress unused-variable warning; prior_state is intentionally
        // not read — overflow is detected by state-apply's pre-check.
        let _ = prior_state;

        // Intent must carry at least the 1-byte discriminator + 8-byte delta.
        if intent.len() < 9 {
            return Err(alloc::string::String::from("intent too short"));
        }

        if intent[0] != 0x00 {
            return Err(alloc::string::String::from("unknown intent discriminator"));
        }

        let delta_bytes: [u8; 8] = match intent[1..9].try_into() {
            Ok(a) => a,
            Err(_) => return Err(alloc::string::String::from("intent delta slice invalid")),
        };
        let delta = i64::from_be_bytes(delta_bytes);

        if delta == 0 {
            return Err(alloc::string::String::from("zero-delta intent rejected"));
        }

        // Event payload is the 8-byte BE delta — matches counter-state-apply's
        // non-genesis payload shape.
        Ok(delta.to_be_bytes().to_vec())
    }
}
