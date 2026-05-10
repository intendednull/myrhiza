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

// Minimal bump allocator. State-apply runs a fresh instance per
// (event, peer); the kernel discards the store after each call so
// we never actually need to free. A bump allocator is the smallest
// thing that satisfies the `extern crate alloc` requirement without
// pulling in float-Display paths from any of the ecosystem crates.
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
        // Bump allocator: never frees. State-apply discards the store
        // after each event, so the cumulative HEAP_SIZE budget bounds
        // the per-event allocation, not the lifetime allocation.
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
    world: "state-apply",
});

struct Component;

export!(Component);

// Hand-rolled fixed-width big-endian wire format. We deliberately do
// NOT use serde+bincode here even though the production state-apply
// path will use canonical bincode at the application level — the
// reason is that linking serde_core into a wasm32-unknown-unknown
// `#![no_std]` binary unconditionally pulls in `<f64 as Display>::fmt`
// (visit_f64 / visit_f32 trait methods, error-message format paths)
// even when the deriving struct uses no float types. Those float
// instructions trip the byte-level float-ban lint per
// determinism.md §5.2 at instantiation time.
//
// State wire format (little-endian-tagged but content is big-endian
// to match the spec's canonical-bincode discipline as closely as a
// hand rolled encoder can):
//   state    = empty | i64-be(value)        // 0 or 8 bytes
//   event    = 0u8 i64-be(by) | 1u8         // 1+8 bytes (Increment) or 1 byte (Reset)
//
// This is a single global counter; no per-key map. The kernel-tier
// acceptance test that exercises this fixture knows the shape and
// asserts the resulting 8 raw bytes decode to 42.

const TAG_INCREMENT: u8 = 0;
const TAG_RESET: u8 = 1;

fn decode_state(bytes: &[u8]) -> Option<i64> {
    if bytes.is_empty() {
        return Some(0);
    }
    let arr: &[u8; 8] = bytes.try_into().ok()?;
    Some(i64::from_be_bytes(*arr))
}

fn encode_state(value: i64) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

impl Guest for Component {
    fn apply(prior_state: Vec<u8>, event: Vec<u8>) -> (Verdict, Vec<u8>) {
        let Some(state) = decode_state(&prior_state) else {
            return (Verdict::Reject(alloc::string::String::from("malformed prior state")), Vec::new());
        };

        let Some((tag, rest)) = event.split_first() else {
            return (Verdict::Reject(alloc::string::String::from("empty event")), Vec::new());
        };

        let new_state = match *tag {
            TAG_INCREMENT => {
                let arr: &[u8; 8] = match rest.try_into() {
                    Ok(a) => a,
                    Err(_) => {
                        return (
                            Verdict::Reject(alloc::string::String::from(
                                "increment payload not 8 bytes",
                            )),
                            Vec::new(),
                        );
                    }
                };
                let by = i64::from_be_bytes(*arr);
                state.saturating_add(by)
            }
            TAG_RESET => 0,
            _ => {
                return (
                    Verdict::Reject(alloc::string::String::from("unknown event tag")),
                    Vec::new(),
                );
            }
        };

        (Verdict::Accept, encode_state(new_state))
    }

    fn state_digest(state: Vec<u8>) -> Vec<u8> {
        // Already canonical big-endian bytes of the i64 counter.
        state
    }
}
