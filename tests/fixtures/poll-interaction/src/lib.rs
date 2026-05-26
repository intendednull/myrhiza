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

// Minimal bump allocator. Interaction runs a fresh instance per view/dispatch;
// the kernel discards the store after each call so we never actually need to
// free. A bump allocator is the smallest thing that satisfies the
// `extern crate alloc` requirement without pulling in float-Display paths
// from any of the ecosystem crates.
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
        // Bump allocator: never frees. Interaction discards the store
        // after each call, so the cumulative HEAP_SIZE budget bounds
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
    world: "interaction",
});

struct Component;

export!(Component);

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
// `dispatch` produces this format; state-propose consumes it.
const DISCRIMINATOR_CREATE_POLL: u8 = 0x00;
const DISCRIMINATOR_VOTE: u8 = 0x01;
const DISCRIMINATOR_END_POLL: u8 = 0x02;

// Materialized poll state. Mirrors poll-state-apply's PollState shape exactly.
// DUPLICATED from `tests/fixtures/poll-state-apply/src/lib.rs::PollState`
// (third copy; poll-state-propose holds the second). The interaction
// component needs to decode `state` to render the view (counts per option,
// option labels, ended flag, "your vote" lookup). Future refactor could
// promote the encoder/decoder into a small `no_std` crate shared between
// state-apply, state-propose, and interaction; out of B-6 scope.
struct PollState {
    options: Vec<String>,
    votes: BTreeMap<[u8; 32], u32>,
    ended: bool,
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

/// Canonical decoder for PollState. Inverse of poll-state-apply's
/// `encode_state`. DUPLICATED from `tests/fixtures/poll-state-apply/src/lib.rs`
/// — see note on struct PollState above. The interaction component does
/// not need `creator` (the view does not surface it directly), so we
/// skip past those bytes rather than store them.
fn decode_state(bytes: &[u8]) -> Option<PollState> {
    let mut cursor: usize = 0;

    // creator (32 raw bytes, no length prefix). Not stored: view does
    // not surface creator identity.
    cursor = cursor.checked_add(32)?;
    if cursor > bytes.len() {
        return None;
    }

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
        options,
        votes,
        ended,
    })
}

// Convert a u32 to its decimal ASCII bytes, appended to `out`.
// Hand-rolled to stay float-free on all code paths (no `core::fmt::Display`
// for numeric types, which transitively pulls in float-Display dispatch
// in the formatting machinery). Mirrors counter-interaction's i64 helper
// at `tests/fixtures/counter-interaction/src/lib.rs:102-156` but specialized
// to u32 (vote counts and option indices are non-negative).
fn u32_to_decimal_bytes(n: u32, out: &mut Vec<u8>) {
    if n == 0 {
        out.push(b'0');
        return;
    }
    // u32::MAX has 10 decimal digits.
    let mut buf = [0u8; 10];
    let mut idx = 0;
    let mut u = n;
    while u > 0 {
        buf[idx] = b'0' + (u % 10) as u8;
        u /= 10;
        idx += 1;
    }
    for i in (0..idx).rev() {
        out.push(buf[i]);
    }
}

// Parse a non-empty decimal string into a u32. Returns None on empty
// input, non-digit bytes, or overflow.
fn parse_u32(s: &str) -> Option<u32> {
    if s.is_empty() {
        return None;
    }
    let mut n: u32 = 0;
    for &b in s.as_bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        n = n.checked_mul(10)?;
        n = n.checked_add((b - b'0') as u32)?;
    }
    Some(n)
}

// Encode `Vec<String>` options into the canonical CreatePoll body layout:
// u64-BE count + (u64-BE label-byte-len + UTF-8 bytes) per entry. Mirrors
// poll-state-apply's `decode_options` inverse exactly.
fn encode_options(options: &[&str], out: &mut Vec<u8>) {
    out.extend_from_slice(&(options.len() as u64).to_be_bytes());
    for label in options {
        out.extend_from_slice(&(label.len() as u64).to_be_bytes());
        out.extend_from_slice(label.as_bytes());
    }
}

// Render `<n> vote` or `<n> votes` with simple if-style pluralization
// per spec §4.1.4. No i18n machinery in v1.
fn render_vote_count(n: u32, out: &mut Vec<u8>) {
    u32_to_decimal_bytes(n, out);
    if n == 1 {
        out.extend_from_slice(b" vote");
    } else {
        out.extend_from_slice(b" votes");
    }
}

impl Guest for Component {
    // Project the poll state as a UTF-8 text block per spec §4.1.4.
    //
    // Layout (literal status line, options with counts, optional "your vote"):
    //
    //   poll: <in-progress | ended>
    //   options:
    //     [0] Yes              (3 votes)
    //     [1] No               (1 vote)
    //     [2] Abstain          (0 votes)
    //   your vote: 1 (No)              # only if peer_state.len() == 32 and voted
    //   your vote: <not voted>         # only if peer_state.len() == 32 and not voted
    //                                  # omitted entirely if peer_state.len() != 32
    //
    // peer_state is the local AuthorPubkey (32 raw bytes per spec §4.1.4
    // harness contract). When peer_state is empty or otherwise not 32 bytes
    // (e.g., harness has not plumbed the contract yet, or some other caller
    // passed an empty slice), the "your vote" line is omitted rather than
    // panicking — the spec mandates tolerance of the placeholder shape.
    fn view(state: Vec<u8>, peer_state: Vec<u8>) -> Vec<u8> {
        let mut out = Vec::new();

        // Decode prior state. Malformed state surfaces a sentinel line so
        // the harness can render something legible without panicking.
        let poll = match decode_state(&state) {
            Some(p) => p,
            None => {
                out.extend_from_slice(b"poll: <invalid state>\n");
                return out;
            }
        };

        // Status line — literal, not a placeholder for runtime data
        // (spec §4.1.4: "is a literal status indicator").
        out.extend_from_slice(b"poll: ");
        if poll.ended {
            out.extend_from_slice(b"ended");
        } else {
            out.extend_from_slice(b"in-progress");
        }
        out.push(b'\n');

        // Options header.
        out.extend_from_slice(b"options:\n");

        // Compute per-option vote counts by iterating state.votes and
        // bucketing by value. counts[i] is the number of votes for
        // options[i]. We iterate state.votes once (O(V)) and produce a
        // Vec<u32> of length state.options.len() (no HashMap).
        let mut counts: Vec<u32> = alloc::vec![0u32; poll.options.len()];
        for opt_idx in poll.votes.values() {
            let i = *opt_idx as usize;
            if i < counts.len() {
                counts[i] = counts[i].saturating_add(1);
            }
            // Out-of-range option_index in votes should not be possible —
            // state-apply rejects Vote events with out-of-range index per
            // spec §4.3. If it appears here, silently drop rather than
            // crash; the view is non-deterministic anyway and a crash is
            // a worse failure mode than a slightly-off count.
        }

        for (i, label) in poll.options.iter().enumerate() {
            out.extend_from_slice(b"  [");
            u32_to_decimal_bytes(i as u32, &mut out);
            out.extend_from_slice(b"] ");
            out.extend_from_slice(label.as_bytes());
            out.extend_from_slice(b" (");
            render_vote_count(counts[i], &mut out);
            out.extend_from_slice(b")\n");
        }

        // Per-peer "your vote" line. Only emit if peer_state is the
        // expected 32-byte AuthorPubkey shape; omit silently otherwise
        // (tolerate placeholder/empty per task brief).
        if peer_state.len() == 32 {
            let mut author = [0u8; 32];
            author.copy_from_slice(&peer_state);
            out.extend_from_slice(b"your vote: ");
            match poll.votes.get(&author) {
                Some(&opt_idx) => {
                    let i = opt_idx as usize;
                    if i < poll.options.len() {
                        // "your vote: 1 (No)" — show index AND label per
                        // spec §4.1.4 sample.
                        u32_to_decimal_bytes(opt_idx, &mut out);
                        out.extend_from_slice(b" (");
                        out.extend_from_slice(poll.options[i].as_bytes());
                        out.push(b')');
                    } else {
                        // Defensively handle out-of-range opt_idx (should
                        // be impossible per state-apply checks, but the
                        // view layer cannot panic).
                        out.extend_from_slice(b"<invalid option>");
                    }
                }
                None => {
                    out.extend_from_slice(b"<not voted>");
                }
            }
            out.push(b'\n');
        }

        out
    }

    // Translate a user action string into an intent for state-propose.
    //
    // Accepted actions (spec §4.5.3):
    //   "create <opt1> <opt2> ..."  → CreatePoll genesis intent
    //   "vote <N>"                  → Vote intent for option N
    //   "end"                       → EndPoll intent
    //
    // Whitespace-tokenized for v1 (spec §4.5.3: "matches counter-interaction's
    // parser style"). Option labels in "create" cannot contain spaces in v1;
    // quoted-string support deferred to B-8 polish.
    //
    // Wire layout per spec §4.5.2 / §4.3:
    //   CreatePoll: [0x00] + canonical-encoded Vec<String> options
    //   Vote:       [0x01] + u32 BE option_index   (5 bytes total)
    //   EndPoll:    [0x02]                          (1 byte total)
    fn dispatch(action: alloc::string::String) -> Result<Vec<u8>, alloc::string::String> {
        let trimmed = action.trim();
        let (verb, rest) = match trimmed.find(' ') {
            Some(i) => (&trimmed[..i], trimmed[i + 1..].trim()),
            None => (trimmed, ""),
        };

        match verb {
            "create" => {
                // Tokenize the rest on whitespace; each token is an option label.
                let options: Vec<&str> = rest.split_ascii_whitespace().collect();
                if options.is_empty() {
                    return Err(alloc::string::String::from(
                        "create: must declare at least one option",
                    ));
                }
                let mut out = Vec::new();
                out.push(DISCRIMINATOR_CREATE_POLL);
                encode_options(&options, &mut out);
                Ok(out)
            }
            "vote" => {
                let option_index = match parse_u32(rest) {
                    Some(n) => n,
                    None => {
                        return Err(alloc::format!("vote: invalid option index: {rest}"));
                    }
                };
                let mut out = Vec::with_capacity(5);
                out.push(DISCRIMINATOR_VOTE);
                out.extend_from_slice(&option_index.to_be_bytes());
                Ok(out)
            }
            "end" => {
                if !rest.is_empty() {
                    return Err(alloc::string::String::from("end: takes no arguments"));
                }
                Ok(alloc::vec![DISCRIMINATOR_END_POLL])
            }
            other => Err(alloc::format!("unknown action: {other}")),
        }
    }

    // Completion handler stubs — the v1 harness does not exercise broadcast
    // or blob-fetch; these are present because the WIT world exports them.
    // Mirrors counter-interaction's pattern at
    // `tests/fixtures/counter-interaction/src/lib.rs:224-232`.
    fn on_broadcast_completion(_token: Vec<u8>, _ok: bool, _err: alloc::string::String) {}

    fn on_blob_fetch_completion(
        _token: Vec<u8>,
        _ok: bool,
        _payload: Vec<u8>,
        _err: alloc::string::String,
    ) {
    }
}
