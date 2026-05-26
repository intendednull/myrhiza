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
// Mirrors `tests/fixtures/counter-state-apply/src/lib.rs`, with one
// load-bearing delta for poll (see "KEY DELTA" below).
//
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
//   deps[..]        : EventId * N   => 40 bytes each (8-byte len + 32-byte hash)
//   hlc             : Hlc           => 12 bytes (u64 wall_ms + u32 logical;
//                                       bincode does not pad)
//   payload_len     : u64           => 8 bytes BE
//   payload[..]     : Vec<u8> serde_bytes contents
//   signature_len   : u64           => 8 bytes BE (=64)
//   signature[..]   : 64 bytes
//
// **KEY DELTA vs counter (spec §6.3)**: counter hard-rejects
// `deps_len != 0` because it only handles linear single-author chains.
// Poll is the first fixture that tolerates non-empty `deps`: voters
// declare `deps = {creator's genesis event hash}` so their per-author
// chains hang off the topic's shared causal anchor. State-apply does
// not enforce DAG topology (kernel's job per B-1); it just skips past
// the deps array to find the payload. The consequence: PAYLOAD_LEN_OFFSET
// is computed dynamically as HLC_OFFSET + 12 + (deps_len * 40),
// NOT a compile-time constant.
//
// GenesisV1 payload layout (founder's seq == 1 with empty prior_state):
//   seed            : [u8; 32]      => 32 raw bytes (no length prefix in
//                                       canonical bincode for fixed arrays)
//   founder_pubkey  : AuthorPubkey, serde_bytes => 8 + 32 = 40 bytes
//   app_payload     : Vec<u8> serde_bytes => 8-byte len + N bytes
//
// Genesis discriminator is `seq == 1 && prior_state.is_empty()`, mirroring
// counter at `tests/fixtures/counter-state-apply/src/lib.rs:191` exactly.

const SEQ_OFFSET: usize = 40;
const PREV_OFFSET: usize = SEQ_OFFSET + 8;
const DEPS_LEN_OFFSET: usize = PREV_OFFSET + 40;
// `Hlc { wall_ms: u64, logical: u32 }` encodes to 12 bytes under canonical
// bincode (fixint BE, no padding) — see `crates/types/src/hlc.rs::tests`.
const HLC_OFFSET_AFTER_DEPS_LEN: usize = DEPS_LEN_OFFSET + 8;
const HLC_LEN: usize = 12;
// Each EventId in the deps array encodes as `8-byte len (=32) + 32-byte hash`
// = 40 bytes.
const EVENT_ID_WIRE_LEN: usize = 40;

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
        // Fixed prefix up to and including deps_len (deps array follows).
        if event.len() < HLC_OFFSET_AFTER_DEPS_LEN {
            return reject("event envelope shorter than fixed prefix");
        }
        // author lives at envelope[0..40] but state-apply only needs it
        // for the EndPoll permission gate (T2). T1 leaves the offset
        // implicit via the SEQ_OFFSET = 40 constant.
        let Some(seq) = read_u64_be(&event, SEQ_OFFSET) else {
            return reject("failed to read seq");
        };
        let Some(deps_len) = read_u64_be(&event, DEPS_LEN_OFFSET) else {
            return reject("failed to read deps_len");
        };
        let deps_len = deps_len as usize;

        // KEY DELTA vs counter: tolerate non-empty deps. Skip past the
        // deps array dynamically to locate the HLC and payload-length
        // fields. counter hard-rejects deps_len != 0 here; poll does not.
        let deps_byte_len = match deps_len.checked_mul(EVENT_ID_WIRE_LEN) {
            Some(n) => n,
            None => return reject("deps length overflow"),
        };
        let hlc_offset = HLC_OFFSET_AFTER_DEPS_LEN + deps_byte_len;
        let payload_len_offset = match hlc_offset.checked_add(HLC_LEN) {
            Some(n) => n,
            None => return reject("hlc offset overflow"),
        };
        // Need 8 bytes for payload_len after the HLC.
        let payload_start = match payload_len_offset.checked_add(8) {
            Some(n) => n,
            None => return reject("payload_len offset overflow"),
        };
        if event.len() < payload_start {
            return reject("event envelope shorter than dynamic prefix");
        }
        let Some(payload_len) = read_u64_be(&event, payload_len_offset) else {
            return reject("failed to read payload_len");
        };
        let payload_len = payload_len as usize;
        let payload_end = match payload_start.checked_add(payload_len) {
            Some(e) => e,
            None => return reject("payload length overflow"),
        };
        let Some(payload) = event.get(payload_start..payload_end) else {
            return reject("payload extends past event bytes");
        };

        if seq == 1 && prior_state.is_empty() {
            // Topic Genesis (founder's seq=1 against never-applied state):
            // T2 will decode the GenesisV1 envelope, extract the CreatePoll
            // body, and materialize the initial PollState. For T1 we only
            // confirm the envelope decodes and reject as unimplemented.
            //
            // Genesis discriminator mirrors counter at
            // counter-state-apply.rs:191. Non-founder seq=1 events have
            // non-empty prior_state and fall through to the non-genesis
            // arm below.
            let _ = GENESIS_APP_PAYLOAD_LEN_OFFSET;
            return reject("unimplemented");
        }

        // Non-genesis events carry a discriminator byte at payload[0]
        // followed by the variant body (Vote = u32 BE option_index;
        // EndPoll = empty; CreatePoll-non-genesis = invalid).
        if payload.is_empty() {
            return reject("event payload empty");
        }
        match payload[0] {
            0x00 => {
                // CreatePoll outside of genesis — T2 will Reject explicitly.
                reject("unimplemented")
            }
            0x01 => {
                // Vote — T2 will decode option_index and update votes.
                reject("unimplemented")
            }
            0x02 => {
                // EndPoll — T2 will check author == state.creator and flip
                // the ended flag.
                reject("unimplemented")
            }
            _ => reject("unimplemented"),
        }
    }

    fn state_digest(state: Vec<u8>) -> Vec<u8> {
        // Placeholder: pass through the raw state bytes. T2 will replace
        // this with a stable digest over the canonical PollState shape
        // (BTreeMap<AuthorPubkey, OptionIndex> + creator + status — spec
        // §4.2). The canonical bincode of PollState is already stable,
        // so identity is the right answer once T2 lands the encoder.
        state
    }
}
