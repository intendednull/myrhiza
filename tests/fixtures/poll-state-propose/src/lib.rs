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

// Spec §4.2 bounds for defense-in-depth validation at the CreatePoll arm.
// state-apply also enforces these; this layer surfaces the error before
// the event is signed.
const MAX_OPTIONS: usize = 16;
const MAX_OPTION_LABEL_LEN_BYTES: usize = 64;

// Intent discriminator bytes per spec §4.5.2.
const DISCRIMINATOR_CREATE_POLL: u8 = 0x00;
const DISCRIMINATOR_VOTE: u8 = 0x01;
const DISCRIMINATOR_END_POLL: u8 = 0x02;

fn err(msg: &str) -> String {
    String::from(msg)
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

// Materialized poll state. Mirrors poll-state-apply's PollState shape
// exactly. Duplicated here because the propose component needs to
// decode `prior_state` (the encoded PollState that state-apply produces)
// to perform defense-in-depth validation against intent bodies. Future
// refactor could promote the encoder/decoder into a small `no_std`
// crate shared between state-apply and state-propose; out of B-6 scope.
struct PollState {
    #[allow(dead_code)]
    creator: [u8; 32],
    options: Vec<String>,
    #[allow(dead_code)]
    votes: BTreeMap<[u8; 32], u32>,
    ended: bool,
}

/// Canonical decoder for PollState. Inverse of state-apply's `encode_state`.
/// DUPLICATED from `tests/fixtures/poll-state-apply/src/lib.rs::decode_state`
/// — see note on struct PollState above.
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

/// Decode `Vec<String>` from the CreatePoll intent body. Same layout as
/// state-apply's `decode_options`: u64-BE count + (u64-BE byte-len +
/// UTF-8 bytes) per entry. DUPLICATED from poll-state-apply — see
/// PollState note above.
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
    if cursor != bytes.len() {
        return None;
    }
    Some(options)
}

// Poll v1 intent vocabulary per spec §4.5.2 (app-internal contract between
// poll-interaction and poll-state-propose — opaque to the kernel):
//
//   intent[0]    = 0x00  CreatePoll
//     intent[1..] = canonical-encoded Vec<String> options (u64-BE count +
//                   (u64-BE label-len + UTF-8 bytes) per entry; matches
//                   state-apply's decode_options layout)
//   intent[0]    = 0x01  Vote
//     intent[1..5] = u32 BE option_index
//   intent[0]    = 0x02  EndPoll
//     intent[1..]  = (empty)
//
// Propose validates each variant against `prior_state` (defense-in-depth:
// state-apply pre-check is the load-bearing check, but propose's Reject
// surfaces the error before the event is signed). On success it returns
// the same byte string back as the event payload — discriminator + body
// layout matches state-apply's expected payload format exactly.
//
// EndPoll asymmetry (spec §4.5.2 final paragraph): propose CANNOT check
// "is the local peer the creator?" because the state-propose WIT world
// has no host-import that exposes the local AuthorPubkey. State-apply
// sees `event.author` because the kernel passes the signed envelope;
// propose only sees prior_state + intent bytes. The future-ABI-gap
// call-out in §4.5.2 is the right place to surface this gap (a future
// `host.local_author()` import); we do not implement a workaround here.
impl Guest for Component {
    fn propose(prior_state: Vec<u8>, intent: Vec<u8>) -> Result<Vec<u8>, String> {
        if intent.is_empty() {
            return Err(err("intent must declare an event kind"));
        }

        match intent[0] {
            DISCRIMINATOR_CREATE_POLL => {
                // CreatePoll is only valid as a genesis intent (state-apply
                // will further enforce seq==1 && prior_state.is_empty()).
                if !prior_state.is_empty() {
                    return Err(err("CreatePoll: only valid as genesis intent"));
                }
                let options_bytes = &intent[1..];
                let options = match decode_options(options_bytes) {
                    Some(o) => o,
                    None => return Err(err("CreatePoll: malformed options encoding")),
                };
                // Defense-in-depth bounds — state-apply also enforces these.
                if options.is_empty() {
                    return Err(err("CreatePoll: must declare ≥1 option"));
                }
                if options.len() > MAX_OPTIONS {
                    return Err(err("CreatePoll: too many options (> MAX_OPTIONS)"));
                }
                for label in &options {
                    if label.len() > MAX_OPTION_LABEL_LEN_BYTES {
                        return Err(err("CreatePoll: option label too long"));
                    }
                }
                // Event payload = intent verbatim (0x00 + canonical options).
                Ok(intent)
            }
            DISCRIMINATOR_VOTE => {
                // Vote requires a materialized poll to validate against.
                let state = match decode_state(&prior_state) {
                    Some(s) => s,
                    None => return Err(err("Vote: prior_state malformed or empty")),
                };
                if state.ended {
                    return Err(err("Vote: poll has ended"));
                }
                // body = u32 BE option_index after the discriminator byte.
                if intent.len() != 5 {
                    return Err(err("Vote: malformed body"));
                }
                let option_index = match read_u32_be(&intent, 1) {
                    Some(v) => v,
                    None => return Err(err("Vote: failed to read option_index")),
                };
                if (option_index as usize) >= state.options.len() {
                    return Err(err("Vote: option_index out of range"));
                }
                // Event payload = intent verbatim (0x01 + u32 BE).
                Ok(intent)
            }
            DISCRIMINATOR_END_POLL => {
                // body must be empty (just the discriminator byte).
                if intent.len() != 1 {
                    return Err(err("EndPoll: malformed body"));
                }
                let state = match decode_state(&prior_state) {
                    Some(s) => s,
                    None => return Err(err("EndPoll: prior_state malformed or empty")),
                };
                if state.ended {
                    return Err(err("EndPoll: poll already ended"));
                }
                // INTENTIONAL: do NOT check `local_author == state.creator`.
                // Propose has no host-import for local author; state-apply
                // enforces creator-only on the signed event. See spec §4.5.2
                // future-ABI-gap call-out.
                Ok(intent)
            }
            _ => Err(err("unknown intent discriminator")),
        }
    }
}
