// Counter — state-propose component.
//
// Per docs/specs/2026-05-26-b-8-sdk-design.md §3.3. Inner attributes
// at file top — see `state.rs` for the rationale.

#![no_std]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]

myrhiza_sdk::myrhiza_app!(state_propose, Component);

use alloc::string::String;
use alloc::vec::Vec;

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
    fn propose(prior_state: Vec<u8>, intent: Vec<u8>) -> Result<Vec<u8>, String> {
        // Suppress unused-variable warning; prior_state is intentionally
        // not read — overflow is detected by state-apply's pre-check.
        let _ = prior_state;

        // Intent must carry at least the 1-byte discriminator + 8-byte delta.
        if intent.len() < 9 {
            return Err(String::from("intent too short"));
        }

        if intent[0] != 0x00 {
            return Err(String::from("unknown intent discriminator"));
        }

        let delta_bytes: [u8; 8] = match intent[1..9].try_into() {
            Ok(a) => a,
            Err(_) => return Err(String::from("intent delta slice invalid")),
        };
        let delta = i64::from_be_bytes(delta_bytes);

        if delta == 0 {
            return Err(String::from("zero-delta intent rejected"));
        }

        // Event payload is the 8-byte BE delta — matches counter-state-apply's
        // non-genesis payload shape.
        Ok(delta.to_be_bytes().to_vec())
    }
}
