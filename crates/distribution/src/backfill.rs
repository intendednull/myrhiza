//! Direct-stream distribution-backfill wire types.
//!
//! Per B-12 spec §14.3 (the corrected transport). A behind peer that
//! hears an advertiser's [`RevocationHeads`](crate::heads::RevocationHeads)
//! / [`PublicationHeads`](crate::heads::PublicationHeads) summary with a
//! head *above* its own **pulls** the missing envelopes by direct-dialing
//! the advertiser over a dedicated direct-stream protocol (mirroring the
//! event-DAG's B-4.4 `request_heads`). This module defines the request and
//! response payloads carried over that stream:
//!
//! - [`DistributionBackfillRequest`] — "send me envelopes with `seq >
//!   from_seq` for this author's `kind` log."
//! - [`DistributionEnvelope`] — one served signed envelope, narrowly typed
//!   so the distribution stream cannot carry an `Event` by construction
//!   (the event-DAG backfill stays on its own protocol — spec §14.2).
//!
//! ## Why a pull, not gossip-push (spec §13/§14)
//!
//! The original B-12 transport re-broadcast missing envelopes on the
//! gossip topic ("ahead peer pushes on hearing a behind summary"). The
//! iroh-tier test proved that cannot catch up a late joiner over real
//! iroh-gossip: the joiner→established gossip path is lazy (Plumtree
//! IHAVE/GRAFT) so the behind peer's summary never reaches an ahead peer,
//! and identical re-broadcasts are content-deduplicated. A point-to-point
//! QUIC dial bypasses both, exactly as the event DAG pulls historical
//! events a late joiner can never receive over gossip. See spec §13.
//!
//! ## Why these live here, not in `crates/network` (spec §14.3 / §12 Q1)
//!
//! [`DistributionEnvelope`] wraps the [`RevocationEvent`] /
//! [`PublicationEvent`] envelopes, which already live in this crate. B-11
//! inverted the crate edge so `crates/network` depends on
//! `crates/distribution` (and distribution carries no dep on network),
//! keeping the package graph a DAG. Putting the backfill request/response
//! types here — beside the envelopes they wrap and the
//! [`heads`](crate::heads) summaries that trigger the pull — and having
//! `crates/network` use them reintroduces no cycle: distribution depends
//! only on `myrhiza-types`.

use serde::{Deserialize, Serialize};

use myrhiza_types::AuthorPubkey;

use crate::publication::PublicationEvent;
use crate::revocation::RevocationEvent;

/// Which per-author distribution log a backfill request targets.
///
/// The two logs have distinct serve semantics (revocation: a contiguous
/// seq range from the full archive; publication: latest-wins, a single
/// envelope), so the request names the ledger explicitly rather than the
/// server inferring it. Per B-12 spec §14.3.
///
/// `Copy` + total order (`Ord`/`PartialOrd`) + `Hash` are derived in
/// addition to the wire traits so the kernel can use it as a `BTreeSet`
/// key (the per-`(author, kind)` in-flight-pull guard, spec §14.4). These
/// are standard fieldless-enum derives and do not affect the wire format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DistributionLogKind {
    /// The author's revocation log (full per-author archive, served as a
    /// contiguous `from_seq+1..=max` range).
    Revocation,
    /// The author's publication log (latest-wins; at most one envelope is
    /// served, and only if its seq exceeds `from_seq`).
    Publication,
}

/// A direct-stream request for the envelopes a behind peer is missing.
///
/// Issued by the behind peer to the advertiser it heard a summary from,
/// over the `DISTRIBUTION_REQUEST_ALPN` protocol (crates/network). Per
/// B-12 spec §14.3.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistributionBackfillRequest {
    /// The author whose log the requester wants backfilled. The serving
    /// peer accepts the request only if it serves this author (else it
    /// drops the stream with a clean EOF).
    pub author: AuthorPubkey,
    /// Which of the author's two logs to serve.
    pub kind: DistributionLogKind,
    /// Exclusive low watermark: the server sends every archived envelope
    /// with `seq > from_seq`. A requester passes its current
    /// `last_observed_seq` (0 if it has no log for the author yet), so the
    /// response is exactly the gap.
    pub from_seq: u64,
}

/// One signed envelope streamed back in response to a
/// [`DistributionBackfillRequest`].
///
/// Narrowly typed to the two distribution ledgers — it *cannot* carry an
/// event-DAG [`Event`](myrhiza_types::Event) by construction, keeping the
/// distribution backfill protocol disjoint from the event-DAG
/// `request_heads` backfill (spec §14.2). The behind peer feeds each
/// received envelope through the existing
/// [`RevocationLog::apply`](crate::revocation::RevocationLog::apply) /
/// [`PublicationLog::apply`](crate::publication::PublicationLog::apply)
/// path, which re-verifies the author signature — so a malicious server
/// cannot inject forged state. Per B-12 spec §14.3.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistributionEnvelope {
    /// A signed revocation envelope from the served range.
    Revocation(RevocationEvent),
    /// The latest signed publication envelope.
    Publication(PublicationEvent),
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use bincode::Options;
    use ed25519_dalek::{Signer, SigningKey};
    use myrhiza_types::{BlobHash, canonical_bincode};

    use super::*;

    fn signing_key() -> SigningKey {
        SigningKey::from_bytes(&[9u8; 32])
    }

    fn signed_revocation(seq: u64) -> RevocationEvent {
        let sk = signing_key();
        let mut ev = RevocationEvent {
            revoked_bundle_hash: BlobHash::from_bytes([0xAB; 32]),
            reason: "compromised".into(),
            revoked_at: 0,
            revocation_seq: seq,
            signature: [0u8; 64],
        };
        ev.signature = sk.sign(&ev.signing_target()).to_bytes();
        ev
    }

    fn signed_publication(seq: u64) -> PublicationEvent {
        let sk = signing_key();
        let mut ev = PublicationEvent {
            manifest_hash: BlobHash::from_bytes([0xCD; 32]),
            version: "1.2.3".into(),
            publication_seq: seq,
            signature: [0u8; 64],
        };
        ev.signature = sk.sign(&ev.signing_target()).to_bytes();
        ev
    }

    #[test]
    fn log_kind_round_trips_via_canonical_bincode() {
        for kind in [
            DistributionLogKind::Revocation,
            DistributionLogKind::Publication,
        ] {
            let bytes = canonical_bincode().serialize(&kind).expect("encode");
            let decoded: DistributionLogKind =
                canonical_bincode().deserialize(&bytes).expect("decode");
            assert_eq!(kind, decoded);
        }
    }

    #[test]
    fn backfill_request_round_trips_via_canonical_bincode() {
        let author = AuthorPubkey::from_bytes([0x11; 32]);
        let req = DistributionBackfillRequest {
            author,
            kind: DistributionLogKind::Revocation,
            from_seq: 42,
        };
        let bytes = canonical_bincode().serialize(&req).expect("encode");
        let decoded: DistributionBackfillRequest =
            canonical_bincode().deserialize(&bytes).expect("decode");
        assert_eq!(req, decoded);
    }

    #[test]
    fn revocation_envelope_round_trips_via_canonical_bincode() {
        let env = DistributionEnvelope::Revocation(signed_revocation(3));
        let bytes = canonical_bincode().serialize(&env).expect("encode");
        let decoded: DistributionEnvelope =
            canonical_bincode().deserialize(&bytes).expect("decode");
        assert_eq!(env, decoded);
    }

    #[test]
    fn publication_envelope_round_trips_via_canonical_bincode() {
        let env = DistributionEnvelope::Publication(signed_publication(7));
        let bytes = canonical_bincode().serialize(&env).expect("encode");
        let decoded: DistributionEnvelope =
            canonical_bincode().deserialize(&bytes).expect("decode");
        assert_eq!(env, decoded);
    }
}
