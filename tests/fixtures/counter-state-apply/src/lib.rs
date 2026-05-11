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

// Hand-rolled byte-offset decoder for canonical bincode Event envelopes.
// We deliberately do NOT pull in the canonical serde+bincode derive
// here: linking `serde_core` into a wasm32-unknown-unknown `#![no_std]`
// binary unconditionally pulls in `<f64 as Display>::fmt` (visit_f64 /
// visit_f32 trait methods, error-message format paths) even when the
// deriving structs use no float types. Those float instructions trip
// the byte-level float-ban lint per determinism.md §5.2 at
// instantiation time.
//
// Canonical bincode (big-endian fixint) Event layout:
//   author          : AuthorPubkey, serde_bytes => 8-byte len (=32) + 32 bytes = 40
//   seq             : u64           => 8 bytes BE
//   prev            : EventId,      serde_bytes => 8-byte len (=32) + 32 bytes = 40
//   deps_len        : u64 list-len  => 8 bytes BE
//   deps[..]        : EventId * N   => 40 bytes each (only N=0 supported)
//   hlc             : Hlc           => 12 bytes (u64 wall_ms + u32 logical;
//                                       bincode does not pad)
//   payload_len     : u64           => 8 bytes BE
//   payload[..]     : Vec<u8> serde_bytes contents
//   signature_len   : u64           => 8 bytes BE (=64)
//   signature[..]   : 64 bytes
//
// For B-1 fixture-authored events deps is always empty. If the kernel
// hands us an envelope with non-empty deps we Reject — the fixture's
// contract is "single-author linear counter".
//
// GenesisV1 payload layout (founder's seq == 1 with empty prior_state):
//   seed            : [u8; 32]      => 32 raw bytes (no length prefix in
//                                       canonical bincode for fixed arrays)
//   founder_pubkey  : AuthorPubkey, serde_bytes => 8 + 32 = 40 bytes
//   app_payload     : Vec<u8> serde_bytes => 8-byte len + N bytes
//
// Non-genesis payload: an 8-byte BE i64 increment. This covers both
// `seq >= 2` AND `seq == 1` for non-founder authors (per-author chains
// start at seq=1; only the founder's seq=1 is the topic Genesis — see
// plan-B-1 spec §4.2 step 3 applicability rule). The discriminator is
// `prior_state.is_empty()`: only the founder's seq=1 sees empty prior
// state, since the kernel applies events in topo order and Genesis is
// the first event applied.
//
// State wire format:
//   The initial state IS the Genesis `app_payload` verbatim — the
//   acceptance test seeds Genesis with `0_i64.to_be_bytes()` so the
//   resulting state is always 8 bytes containing the running i64
//   counter in big-endian.

const SEQ_OFFSET: usize = 40;
const PREV_OFFSET: usize = SEQ_OFFSET + 8;
const DEPS_LEN_OFFSET: usize = PREV_OFFSET + 40;
// Assumes deps_len == 0 (B-1 fixture contract).
const HLC_OFFSET: usize = DEPS_LEN_OFFSET + 8;
// `Hlc { wall_ms: u64, logical: u32 }` encodes to 12 bytes under canonical
// bincode (fixint BE, no padding) — see `crates/types/src/hlc.rs::tests`.
const PAYLOAD_LEN_OFFSET: usize = HLC_OFFSET + 12;

// GenesisV1 payload offsets.
const GENESIS_APP_PAYLOAD_LEN_OFFSET: usize = 32 + 40; // seed + founder_pubkey

fn reject(msg: &str) -> (Verdict, Vec<u8>) {
    (
        Verdict::Reject(alloc::string::String::from(msg)),
        Vec::new(),
    )
}

fn read_u64_be(bytes: &[u8], offset: usize) -> Option<u64> {
    let end = offset.checked_add(8)?;
    let slice = bytes.get(offset..end)?;
    let arr: [u8; 8] = slice.try_into().ok()?;
    Some(u64::from_be_bytes(arr))
}

impl Guest for Component {
    fn apply(prior_state: Vec<u8>, event: Vec<u8>) -> (Verdict, Vec<u8>) {
        if event.len() < PAYLOAD_LEN_OFFSET + 8 {
            return reject("event envelope shorter than fixed prefix");
        }
        let Some(seq) = read_u64_be(&event, SEQ_OFFSET) else {
            return reject("failed to read seq");
        };
        let Some(deps_len) = read_u64_be(&event, DEPS_LEN_OFFSET) else {
            return reject("failed to read deps_len");
        };
        if deps_len != 0 {
            // B-1 fixture only handles linear single-author chains.
            return reject("non-empty deps not supported");
        }
        let Some(payload_len) = read_u64_be(&event, PAYLOAD_LEN_OFFSET) else {
            return reject("failed to read payload_len");
        };
        let payload_len = payload_len as usize;
        let payload_start = PAYLOAD_LEN_OFFSET + 8;
        let payload_end = match payload_start.checked_add(payload_len) {
            Some(e) => e,
            None => return reject("payload length overflow"),
        };
        let Some(payload) = event.get(payload_start..payload_end) else {
            return reject("payload extends past event bytes");
        };

        if seq == 1 && prior_state.is_empty() {
            // Topic Genesis (founder's seq=1 against never-applied state):
            // decode GenesisV1 and return app_payload as initial state.
            // Non-founder seq=1 events have non-empty prior_state (the
            // post-Genesis counter) and fall through to the i64
            // increment path — see plan-B-1 spec §12.1.1.
            if payload.len() < GENESIS_APP_PAYLOAD_LEN_OFFSET + 8 {
                return reject("genesis payload shorter than fixed prefix");
            }
            let Some(app_len) = read_u64_be(payload, GENESIS_APP_PAYLOAD_LEN_OFFSET) else {
                return reject("failed to read genesis app_payload_len");
            };
            let app_len = app_len as usize;
            let start = GENESIS_APP_PAYLOAD_LEN_OFFSET + 8;
            let end = match start.checked_add(app_len) {
                Some(e) => e,
                None => return reject("genesis app_payload length overflow"),
            };
            let Some(app_payload) = payload.get(start..end) else {
                return reject("genesis app_payload extends past payload");
            };
            return (Verdict::Accept, app_payload.to_vec());
        }

        // Non-genesis (either seq >= 2, OR seq == 1 from a non-founder
        // author whose chain head is being applied against the existing
        // post-Genesis state): payload is 8-byte BE i64 increment,
        // prior_state is 8-byte BE i64 counter. Sum and return.
        if payload.len() != 8 {
            return reject("non-genesis payload must be 8 bytes");
        }
        if prior_state.len() != 8 {
            return reject("prior_state must be 8 bytes (i64)");
        }
        let increment_arr: [u8; 8] = match payload.try_into() {
            Ok(a) => a,
            Err(_) => return reject("increment slice-to-array failed"),
        };
        let current_arr: [u8; 8] = match prior_state.as_slice().try_into() {
            Ok(a) => a,
            Err(_) => return reject("prior_state slice-to-array failed"),
        };
        let increment = i64::from_be_bytes(increment_arr);
        let current = i64::from_be_bytes(current_arr);
        let new = current.saturating_add(increment);
        (Verdict::Accept, new.to_be_bytes().to_vec())
    }

    fn state_digest(state: Vec<u8>) -> Vec<u8> {
        // State is the raw bytes the kernel persists — the acceptance
        // test seeds Genesis with the canonical 8-byte BE i64 zero, so
        // this is already a stable digest.
        state
    }
}
