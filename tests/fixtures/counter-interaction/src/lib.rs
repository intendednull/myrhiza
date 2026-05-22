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

// Counter v1 intent vocabulary (app-internal contract between
// counter-interaction and counter-state-propose — opaque to the kernel):
//
//   intent[0]    = 0x00  // Increment discriminator
//   intent[1..9] = i64 BE delta
//
// dispatch produces this format; state-propose consumes it.

// Convert an i64 to its decimal ASCII bytes, appended to `out`.
// Hand-rolled to stay float-free on all code paths, including
// the i64::MIN edge case which overflows naive negation.
fn i64_to_decimal_bytes(n: i64, out: &mut Vec<u8>) {
    if n == 0 {
        out.push(b'0');
        return;
    }
    if n < 0 {
        // i64::MIN cannot be negated as i64 (overflow), so cast through u64.
        let abs: u64 = if n == i64::MIN {
            9_223_372_036_854_775_808_u64
        } else {
            (-n) as u64
        };
        let mut buf = [0u8; 20];
        let mut idx = 0;
        let mut u = abs;
        while u > 0 {
            buf[idx] = b'0' + (u % 10) as u8;
            u /= 10;
            idx += 1;
        }
        out.push(b'-');
        for i in (0..idx).rev() {
            out.push(buf[i]);
        }
    } else {
        let mut buf = [0u8; 20];
        let mut idx = 0;
        let mut u = n as u64;
        while u > 0 {
            buf[idx] = b'0' + (u % 10) as u8;
            u /= 10;
            idx += 1;
        }
        for i in (0..idx).rev() {
            out.push(buf[i]);
        }
    }
}

// Parse a non-empty decimal string into an i64. Returns None on empty
// input, non-digit bytes, or overflow.
fn parse_i64(s: &str) -> Option<i64> {
    if s.is_empty() {
        return None;
    }
    let mut n: i64 = 0;
    for &b in s.as_bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        n = n.checked_mul(10)?;
        n = n.checked_add((b - b'0') as i64)?;
    }
    Some(n)
}

impl Guest for Component {
    // Project the counter state as a human-readable UTF-8 line.
    //
    // peer_state is ignored for v1 (Choice D: peer-state is read-only,
    // always empty for counter). state is 8-byte BE i64; anything else
    // yields the invalid-state sentinel so the harness can display it
    // without panic.
    fn view(state: Vec<u8>, _peer_state: Vec<u8>) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"counter: ");
        if state.len() == 8 {
            let arr: [u8; 8] = match state.as_slice().try_into() {
                Ok(a) => a,
                Err(_) => {
                    out.extend_from_slice(b"<invalid state>");
                    out.push(b'\n');
                    return out;
                }
            };
            let n = i64::from_be_bytes(arr);
            i64_to_decimal_bytes(n, &mut out);
        } else {
            out.extend_from_slice(b"<invalid state>");
        }
        out.push(b'\n');
        out
    }

    // Translate a user action string into an intent for state-propose.
    //
    // Accepted actions:
    //   "inc"    → delta = +1
    //   "inc N"  → delta = +N  (N positive integer)
    //   "dec"    → delta = -1
    //   "dec N"  → delta = -N  (N positive integer)
    //
    // Returns Ok([0x00, delta_be_i64...]) on success; Err(...) otherwise.
    fn dispatch(action: alloc::string::String) -> Result<Vec<u8>, alloc::string::String> {
        let trimmed = action.trim();
        let (verb, rest) = match trimmed.find(' ') {
            Some(i) => (&trimmed[..i], trimmed[i + 1..].trim()),
            None => (trimmed, ""),
        };
        let magnitude: i64 = if rest.is_empty() {
            1
        } else {
            match parse_i64(rest) {
                Some(n) if n > 0 => n,
                _ => {
                    return Err(alloc::format!("invalid argument: {rest}"));
                }
            }
        };
        let delta: i64 = match verb {
            "inc" => magnitude,
            "dec" => -magnitude,
            other => return Err(alloc::format!("unknown action: {other}")),
        };
        let mut out = Vec::with_capacity(9);
        out.push(0x00_u8);
        out.extend_from_slice(&delta.to_be_bytes());
        Ok(out)
    }

    // Completion handler stubs — the v1 harness does not exercise broadcast
    // or blob-fetch; these are present because the WIT world exports them.
    fn on_broadcast_completion(_token: Vec<u8>, _ok: bool, _err: alloc::string::String) {}

    fn on_blob_fetch_completion(
        _token: Vec<u8>,
        _ok: bool,
        _payload: Vec<u8>,
        _err: alloc::string::String,
    ) {
    }
}
