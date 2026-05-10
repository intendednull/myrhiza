// `wit-bindgen` 0.30 emits `unsafe fn` post-return helpers whose bodies call
// unsafe ops directly. Edition 2024's `unsafe_op_in_unsafe_fn` lint defaults
// to deny, so we relax it locally for the generated bindings. The wit-bindgen
// upstream addresses this in newer releases; we pin 0.30 per the plan.
#![allow(unsafe_op_in_unsafe_fn)]
#![no_std]
extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use bincode::Options;
use serde::{Deserialize, Serialize};

wit_bindgen::generate!({
    world: "state-apply",
});

struct Component;

export!(Component);

#[derive(Default, Serialize, Deserialize)]
struct CounterState {
    by_key: BTreeMap<String, i64>,
}

#[derive(Serialize, Deserialize)]
enum CounterEvent {
    Increment { key: String, by: i64 },
    Reset { key: String },
}

fn opts() -> impl bincode::Options {
    bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .with_big_endian()
}

impl Guest for Component {
    fn apply(prior_state: Vec<u8>, event: Vec<u8>) -> (Verdict, Vec<u8>) {
        let mut state: CounterState = if prior_state.is_empty() {
            CounterState::default()
        } else {
            match opts().deserialize(&prior_state) {
                Ok(s) => s,
                Err(_) => {
                    return (
                        Verdict::Reject("malformed prior state".into()),
                        Vec::new(),
                    );
                }
            }
        };

        let evt: CounterEvent = match opts().deserialize(&event) {
            Ok(e) => e,
            Err(_) => return (Verdict::Reject("malformed event".into()), Vec::new()),
        };

        match evt {
            CounterEvent::Increment { key, by } => {
                let entry = state.by_key.entry(key).or_insert(0);
                *entry = entry.saturating_add(by);
            }
            CounterEvent::Reset { key } => {
                state.by_key.remove(&key);
            }
        }

        match opts().serialize(&state) {
            Ok(bytes) => (Verdict::Accept, bytes),
            Err(_) => (Verdict::Reject("encode failure".into()), Vec::new()),
        }
    }

    fn state_digest(state: Vec<u8>) -> Vec<u8> {
        // Already canonical bincode of CounterState. Hash externally.
        state
    }
}
