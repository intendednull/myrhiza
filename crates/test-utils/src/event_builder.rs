//! `EventBuilder` — sign canonical Event envelopes for tests.
//!
//! Re-exports [`myrhiza_kernel::identity::AuthorKeypair`] for ergonomics.
//! Do NOT redefine `AuthorKeypair` here.

use std::collections::BTreeSet;

use bincode::Options;
pub use myrhiza_kernel::identity::AuthorKeypair;
use myrhiza_types::{BundleHash, Event, EventHash, GenesisV1, Hlc, canonical_bincode};

/// Build signed [`Event`] envelopes for tests.
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
    /// The payload is the canonical-bincode encoding of [`GenesisV1`]. The
    /// `_app_bundle_hash` and `_topic_name` parameters are accepted for
    /// signature symmetry with `myrhiza_types::Topic::derive` but are not
    /// used here — they bind the genesis to its topic via the kernel, not
    /// the envelope.
    ///
    /// # Panics
    /// Panics if canonical-bincode encoding of [`GenesisV1`] fails. The
    /// encoding is infallible for the well-typed fields populated here, so
    /// the panic is structurally unreachable.
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
    /// Payload bytes are opaque to the builder; callers encode their own
    /// application format. `deps` are merged-in DAG dependencies beyond the
    /// implicit `prev` link.
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
/// Convenience wrapper around [`canonical_bincode`]; the encoding is the same
/// bytes the network layer hashes to derive [`Event::wire_hash`].
///
/// # Panics
/// Panics if canonical-bincode encoding of the [`Event`] fails. Encoding is
/// infallible for well-typed Events, so this panic is structurally
/// unreachable.
#[must_use]
#[allow(clippy::expect_used)]
pub fn canonical_envelope(event: &Event) -> Vec<u8> {
    canonical_bincode()
        .serialize(event)
        .expect("canonical bincode of Event is infallible")
}

/// Encode an `i64` increment as the 8-byte BE payload the counter fixture
/// expects.
///
/// The counter test app interprets its payload as a single big-endian
/// `i64` delta; this helper keeps that contract in one place.
#[must_use]
pub fn counter_increment_payload(delta: i64) -> Vec<u8> {
    delta.to_be_bytes().to_vec()
}
