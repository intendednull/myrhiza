//! Over-importer fixture: imports `host-non-deterministic.random`
//! which the state-apply linker per architecture.md §3.5 does NOT
//! bind. Component instantiation must fail at link time.
//!
//! See `wit/world.wit` for the import declaration and the matching
//! kernel-tier acceptance test
//! `crates/kernel/tests/acceptance.rs::capability_gating_rejects_non_deterministic_import`.

#![allow(unsafe_op_in_unsafe_fn)]
#![no_std]
#![no_main]
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};

wit_bindgen::generate!({
    world: "state-apply",
});

struct Component;

export!(Component);

impl Guest for Component {
    fn apply(_prior_state: Vec<u8>, _event: Vec<u8>) -> (Verdict, Vec<u8>) {
        // Force the import to be retained even under aggressive
        // dead-code elimination: actually call into the
        // non-deterministic random helper.
        let _ = myrhiza::kernel::host_non_deterministic::random(8);
        (Verdict::Accept, Vec::new())
    }

    fn state_digest(state: Vec<u8>) -> Vec<u8> {
        state
    }
}

// Bump allocator + panic handler — see counter-state-apply for the
// rationale. State-apply runs a fresh instance per event so the
// allocator never frees.
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

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static GLOBAL: BumpAlloc = BumpAlloc;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

// Suppress unused-import / dead-code warnings when nothing else
// references String at this scope (some wit-bindgen versions emit
// String in their generated bindings; some don't).
#[allow(dead_code)]
fn _keep_string(_: String) {}
