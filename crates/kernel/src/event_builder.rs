//! `EventBuilder` — sign canonical Event envelopes.
//!
//! Canonical home: `crates/kernel`. `crates/test-utils` re-exports for
//! backward-compat.

use std::collections::BTreeSet;

use bincode::Options;
use myrhiza_types::{BundleHash, Event, EventHash, GenesisV1, Hlc, canonical_bincode};

pub use crate::identity::AuthorKeypair;

/// Build signed [`Event`] envelopes.
///
/// Holds a borrowed reference to an [`AuthorKeypair`] and stamps each event
/// with `body_hash → Ed25519 signature` per `convergence.md` §4.2 step 1.
/// Payloads are opaque — the builder does not interpret application bytes.
pub struct EventBuilder<'a> {
    /// Author keypair used to sign every event produced by this builder.
    pub author_key: &'a AuthorKeypair,
}

impl<'a> EventBuilder<'a> {
    /// Construct a builder around the given author keypair.
    #[must_use]
    pub fn new(author_key: &'a AuthorKeypair) -> Self {
        Self { author_key }
    }

    /// Build a signed Genesis event (seq=1, prev=ZERO, no deps).
    ///
    /// # Panics
    /// Panics if canonical-bincode encoding of [`GenesisV1`] fails (infallible
    /// for well-typed fields).
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn genesis(
        &self,
        _app_bundle_hash: &BundleHash,
        seed: [u8; 32],
        _topic_name: &str,
        app_payload: Vec<u8>,
    ) -> Event {
        let payload = GenesisV1 {
            seed,
            founder_pubkey: self.author_key.author,
            app_payload,
        };
        let payload_bytes = canonical_bincode().serialize(&payload).expect("encode");
        let body = Event {
            author: self.author_key.author,
            seq: 1,
            prev: EventHash::ZERO,
            deps: BTreeSet::new(),
            hlc: Hlc {
                wall_ms: 0,
                logical: 0,
            },
            payload: payload_bytes,
            signature: [0; 64],
        };
        self.sign(body)
    }

    /// Build a signed event linked to `prev` (seq = `prev.seq + 1`).
    ///
    /// Payload bytes are opaque to the builder.
    #[must_use]
    pub fn next(&self, prev: &Event, deps: BTreeSet<EventHash>, payload: Vec<u8>) -> Event {
        let body = Event {
            author: self.author_key.author,
            seq: prev.seq + 1,
            prev: prev.wire_hash(),
            deps,
            hlc: Hlc {
                wall_ms: 0,
                logical: 0,
            },
            payload,
            signature: [0; 64],
        };
        self.sign(body)
    }

    fn sign(&self, body: Event) -> Event {
        let body_hash = body.hash_signed_body();
        let signature = self.author_key.sign_body_hash(body_hash);
        Event { signature, ..body }
    }
}

/// Canonical-encode an [`Event`] into the wire envelope bytes.
///
/// # Panics
/// Panics if canonical-bincode encoding fails (infallible for well-typed Events).
#[must_use]
#[allow(clippy::expect_used)]
pub fn canonical_envelope(event: &Event) -> Vec<u8> {
    canonical_bincode()
        .serialize(event)
        .expect("canonical bincode of Event is infallible")
}

/// Encode an `i64` increment as the 8-byte BE payload the counter fixture
/// expects.
#[must_use]
pub fn counter_increment_payload(delta: i64) -> Vec<u8> {
    delta.to_be_bytes().to_vec()
}
