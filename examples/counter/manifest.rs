//! Counter app manifest authored via the `manifest!` macro.
//!
//! Per docs/specs/2026-05-26-b-8-sdk-design.md §3.2. The macro
//! desugars to a fully-constructed and canonicalized `Manifest`
//! struct — schema validation surfaces at compile time via the
//! macro's expansion against `myrhiza_manifest::schema::Manifest`.

use myrhiza_sdk::prelude::*;

/// Build the canonical counter manifest.
#[must_use]
pub fn build() -> Manifest {
    myrhiza_sdk::manifest! {
        app {
            name: "counter",
            version: "0.1.0",
            description: "Shared counter MVP demo app",
            author_class: third_party,
        }
        abi {
            kernel_major: 1,
            kernel_minor_min: 0,
            state_digest_format: bincode13,
        }
        capabilities {
            deterministic_helpers: ["host.hash", "host.log"],
        }
        components {
            state_apply: "components/state-apply.wasm",
            state_propose: "components/state-propose.wasm",
            interaction: "components/interaction.wasm",
        }
    }
}
