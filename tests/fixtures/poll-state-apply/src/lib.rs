// `wit-bindgen` 0.30 emits `unsafe fn` post-return helpers whose bodies call
// unsafe ops directly. Edition 2024's `unsafe_op_in_unsafe_fn` lint defaults
// to deny, so we relax it locally for the generated bindings. The wit-bindgen
// upstream addresses this in newer releases; we pin 0.30 per the plan.
#![allow(unsafe_op_in_unsafe_fn)]
#![no_std]
#![no_main]
extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
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

const AUTHOR_OFFSET: usize = 8; // author bytes start at offset 8 (after 8-byte len prefix)
const AUTHOR_LEN: usize = 32;
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
const GENESIS_SEED_LEN: usize = 32;
const GENESIS_FOUNDER_PUBKEY_OFFSET: usize = GENESIS_SEED_LEN;
const GENESIS_FOUNDER_PUBKEY_BYTES_OFFSET: usize = GENESIS_FOUNDER_PUBKEY_OFFSET + 8; // skip 8-byte len prefix
const GENESIS_APP_PAYLOAD_LEN_OFFSET: usize = 32 + 40; // seed + founder_pubkey (len-prefixed)

// Spec §4.2 bounds for genesis-time validation.
const MAX_OPTIONS: usize = 16;
const MAX_OPTION_LABEL_LEN_BYTES: usize = 64;

// Event discriminator bytes per spec §4.3.
const DISCRIMINATOR_CREATE_POLL: u8 = 0x00;
const DISCRIMINATOR_VOTE: u8 = 0x01;
const DISCRIMINATOR_END_POLL: u8 = 0x02;

fn reject(msg: &str) -> (Verdict, Vec<u8>) {
    (Verdict::Reject(String::from(msg)), Vec::new())
}

fn read_u64_be(bytes: &[u8], offset: usize) -> Option<u64> {
    let end = offset.checked_add(8)?;
    let slice = bytes.get(offset..end)?;
    let arr: [u8; 8] = slice.try_into().ok()?;
    Some(u64::from_be_bytes(arr))
}

fn read_u32_be(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    let slice = bytes.get(offset..end)?;
    let arr: [u8; 4] = slice.try_into().ok()?;
    Some(u32::from_be_bytes(arr))
}

/// Materialized poll state. NOT serialized via serde — the canonical
/// encoder below hand-rolls a stable byte layout.
struct PollState {
    creator: [u8; 32],
    options: Vec<String>,
    votes: BTreeMap<[u8; 32], u32>,
    ended: bool,
}

/// Canonical encoder for PollState. ~30 LOC per spec §4.5.1 last paragraph.
///
/// Layout:
///   creator       : 32 raw bytes (fixed-size, no length prefix)
///   options_len   : u64 BE
///   options[i]    : u64 BE byte-len + raw UTF-8 bytes
///   votes_len     : u64 BE (BTreeMap iterated in key-sorted order —
///                          this IS the determinism guarantee per §6.1)
///   votes[i]      : 32 bytes author + u32 BE option_index
///   ended         : 1 byte (0 or 1)
fn encode_state(state: &PollState) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&state.creator);
    out.extend_from_slice(&(state.options.len() as u64).to_be_bytes());
    for opt in &state.options {
        out.extend_from_slice(&(opt.len() as u64).to_be_bytes());
        out.extend_from_slice(opt.as_bytes());
    }
    out.extend_from_slice(&(state.votes.len() as u64).to_be_bytes());
    // BTreeMap yields entries sorted by key bytes — this is the
    // load-bearing determinism property per spec §6.1. Two peers with
    // the same `votes` set always produce identical encoded bytes.
    for (author, opt_idx) in &state.votes {
        out.extend_from_slice(author);
        out.extend_from_slice(&opt_idx.to_be_bytes());
    }
    out.push(if state.ended { 1 } else { 0 });
    out
}

/// Canonical decoder. Inverse of `encode_state`. On any malformed prefix
/// returns `None`; callers convert to `Reject`.
fn decode_state(bytes: &[u8]) -> Option<PollState> {
    let mut cursor: usize = 0;

    // creator
    let creator_end = cursor.checked_add(32)?;
    let creator_slice = bytes.get(cursor..creator_end)?;
    let mut creator = [0u8; 32];
    creator.copy_from_slice(creator_slice);
    cursor = creator_end;

    // options vec
    let options_len = read_u64_be(bytes, cursor)? as usize;
    cursor = cursor.checked_add(8)?;
    let mut options: Vec<String> = Vec::new();
    for _ in 0..options_len {
        let label_len = read_u64_be(bytes, cursor)? as usize;
        cursor = cursor.checked_add(8)?;
        let label_end = cursor.checked_add(label_len)?;
        let label_bytes = bytes.get(cursor..label_end)?;
        // Validate UTF-8 without using format! — core::str::from_utf8 is float-free.
        let label_str = core::str::from_utf8(label_bytes).ok()?;
        options.push(String::from(label_str));
        cursor = label_end;
    }

    // votes map
    let votes_len = read_u64_be(bytes, cursor)? as usize;
    cursor = cursor.checked_add(8)?;
    let mut votes: BTreeMap<[u8; 32], u32> = BTreeMap::new();
    for _ in 0..votes_len {
        let author_end = cursor.checked_add(32)?;
        let author_slice = bytes.get(cursor..author_end)?;
        let mut author = [0u8; 32];
        author.copy_from_slice(author_slice);
        cursor = author_end;
        let opt_idx = read_u32_be(bytes, cursor)?;
        cursor = cursor.checked_add(4)?;
        votes.insert(author, opt_idx);
    }

    // ended
    let ended_byte = *bytes.get(cursor)?;
    cursor = cursor.checked_add(1)?;
    let ended = match ended_byte {
        0 => false,
        1 => true,
        _ => return None,
    };

    // Trailing-bytes check: canonical encoding must be exact.
    if cursor != bytes.len() {
        return None;
    }

    Some(PollState {
        creator,
        options,
        votes,
        ended,
    })
}

/// Decode `Vec<String>` from the CreatePoll body. Returns `(options, ())`
/// or `None` if malformed. Layout: u64-BE count + (u64-BE byte-len + bytes)
/// per entry. Identical to the in-state encoding but standalone for
/// the genesis-payload path.
fn decode_options(bytes: &[u8]) -> Option<Vec<String>> {
    let mut cursor: usize = 0;
    let count = read_u64_be(bytes, cursor)? as usize;
    cursor = cursor.checked_add(8)?;
    let mut options: Vec<String> = Vec::new();
    for _ in 0..count {
        let label_len = read_u64_be(bytes, cursor)? as usize;
        cursor = cursor.checked_add(8)?;
        let label_end = cursor.checked_add(label_len)?;
        let label_bytes = bytes.get(cursor..label_end)?;
        let label_str = core::str::from_utf8(label_bytes).ok()?;
        options.push(String::from(label_str));
        cursor = label_end;
    }
    // Trailing-bytes check.
    if cursor != bytes.len() {
        return None;
    }
    Some(options)
}

impl Guest for Component {
    fn apply(prior_state: Vec<u8>, event: Vec<u8>) -> (Verdict, Vec<u8>) {
        // Fixed prefix up to and including deps_len (deps array follows).
        if event.len() < HLC_OFFSET_AFTER_DEPS_LEN {
            return reject("event envelope shorter than fixed prefix");
        }
        // event.author lives at envelope[AUTHOR_OFFSET..AUTHOR_OFFSET+AUTHOR_LEN]
        // (8-byte serde_bytes length prefix + 32 pubkey bytes; counter:101).
        let author_slice = match event.get(AUTHOR_OFFSET..AUTHOR_OFFSET + AUTHOR_LEN) {
            Some(s) => s,
            None => return reject("failed to read event author"),
        };
        let mut event_author = [0u8; 32];
        event_author.copy_from_slice(author_slice);

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
            // Topic Genesis: decode GenesisV1 envelope (seed + founder_pubkey
            // + app_payload) and materialize initial PollState from the
            // CreatePoll body wrapped inside.
            //
            // Genesis discriminator mirrors counter at
            // counter-state-apply.rs:191. Non-founder seq=1 events have
            // non-empty prior_state and fall through to the non-genesis
            // arm below.
            if payload.len() < GENESIS_APP_PAYLOAD_LEN_OFFSET + 8 {
                return reject("genesis payload shorter than fixed prefix");
            }
            // Extract founder_pubkey from the GenesisV1 envelope.
            // Layout: seed(32) + len(8) + pubkey(32) = 72 bytes before app_payload.
            let founder_slice = match payload
                .get(GENESIS_FOUNDER_PUBKEY_BYTES_OFFSET..GENESIS_APP_PAYLOAD_LEN_OFFSET)
            {
                Some(s) => s,
                None => return reject("failed to read founder_pubkey"),
            };
            let mut creator = [0u8; 32];
            creator.copy_from_slice(founder_slice);

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

            // app_payload is `0x00 ‖ canonical(options)` per spec §4.3.
            if app_payload.is_empty() {
                return reject("genesis app_payload empty");
            }
            if app_payload[0] != DISCRIMINATOR_CREATE_POLL {
                return reject("genesis must be CreatePoll discriminator");
            }
            let options_bytes = &app_payload[1..];
            let options = match decode_options(options_bytes) {
                Some(o) => o,
                None => return reject("CreatePoll: malformed options encoding"),
            };
            if options.is_empty() {
                return reject("CreatePoll: must declare ≥1 option");
            }
            if options.len() > MAX_OPTIONS {
                return reject("CreatePoll: must declare 1..=MAX_OPTIONS");
            }
            for label in &options {
                if label.len() > MAX_OPTION_LABEL_LEN_BYTES {
                    return reject("CreatePoll: option label too long");
                }
            }

            let state = PollState {
                creator,
                options,
                votes: BTreeMap::new(),
                ended: false,
            };
            return (Verdict::Accept, encode_state(&state));
        }

        // Non-genesis path: prior_state must decode into a PollState; payload
        // carries a single-byte discriminator followed by the variant body.
        if payload.is_empty() {
            return reject("event payload empty");
        }
        let mut state = match decode_state(&prior_state) {
            Some(s) => s,
            None => return reject("prior_state malformed"),
        };

        match payload[0] {
            DISCRIMINATOR_CREATE_POLL => {
                // CreatePoll is only valid as a genesis event (handled above).
                reject("CreatePoll: only valid as genesis")
            }
            DISCRIMINATOR_VOTE => {
                if state.ended {
                    return reject("Vote: poll has ended");
                }
                // body = u32 BE option_index after the discriminator byte.
                if payload.len() != 5 {
                    return reject("Vote: malformed body");
                }
                let option_index = match read_u32_be(payload, 1) {
                    Some(v) => v,
                    None => return reject("Vote: failed to read option_index"),
                };
                if (option_index as usize) >= state.options.len() {
                    return reject("Vote: option_index out of range");
                }
                // Last-vote-wins via BTreeMap insert (§4.1.2): overwrite
                // any existing entry for this author. BTreeMap key-sorted
                // iteration on encode keeps the digest deterministic.
                state.votes.insert(event_author, option_index);
                (Verdict::Accept, encode_state(&state))
            }
            DISCRIMINATOR_END_POLL => {
                // Permission gate (§4.1.3): only the creator may end the poll.
                if event_author != state.creator {
                    return reject("EndPoll: not poll creator");
                }
                // body must be empty (just the discriminator byte).
                if payload.len() != 1 {
                    return reject("EndPoll: malformed body");
                }
                state.ended = true;
                (Verdict::Accept, encode_state(&state))
            }
            _ => reject("unknown discriminator"),
        }
    }

    fn state_digest(state: Vec<u8>) -> Vec<u8> {
        // Identity. The canonical encoding produced by `encode_state` IS
        // already a stable digest: BTreeMap iteration is key-sorted so two
        // peers with the same vote set produce identical bytes. Mirrors
        // counter:239-244 and echo:217-220.
        state
    }
}
