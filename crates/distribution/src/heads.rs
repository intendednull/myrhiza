//! Distribution-log head summaries for stale-network backfill.
//!
//! Per B-12 spec §3.2 / §14.1 (the corrected pull transport; §14
//! supersedes the original push design — see §13 for why push could not
//! catch up a late joiner over real iroh-gossip). A peer advertises its
//! `last_observed_seq` for an author's revocation or publication log on
//! the existing per-author derived topic. A *behind* peer hearing a
//! summary whose head is *above* its own **dials the advertiser** (the
//! `advertiser` field below) over the direct-stream
//! `request_distribution` protocol and **pulls** the missing signed
//! envelopes from the advertiser's archive (revocation: the contiguous
//! range; publication: the single latest envelope, latest-wins). Point-to-
//! point QUIC bypasses the Plumtree joiner→established gossip asymmetry
//! that defeated push (spec §13). The pulled envelopes re-enter the apply
//! path; the receiver's monotonic-seq check at the
//! [`crate::revocation::RevocationLog::apply`] /
//! [`crate::publication::PublicationLog::apply`] edge drops duplicates, so
//! the pull is idempotent.
//!
//! ## Why these live here, not in `crates/network` (resolves spec §12 Q1)
//!
//! These payload types are carried by the `GossipMessage::{RevocationHeads,
//! PublicationHeads}` variants in `crates/network`, exactly as the
//! [`crate::revocation::RevocationEvent`] /
//! [`crate::publication::PublicationEvent`] envelopes are carried by the
//! `Revocation`/`Publication` variants. B-11 inverted the crate edge so
//! `crates/network` depends on `crates/distribution` (and distribution
//! carries NO dep on network), keeping the package graph a DAG. Putting
//! the summaries here and re-exporting them through the network enum
//! mirrors that inversion and reintroduces no cycle: distribution depends
//! only on `myrhiza-types` (which provides [`AuthorPubkey`]).
//!
//! ## Why unsigned (spec §3.6 / §14.1)
//!
//! Unlike [`crate::revocation::RevocationEvent`], these summaries carry no
//! signature. The security-critical artefacts are the author-signed
//! envelopes, verified at the dispatch edge on apply. A summary can only
//! ever *trigger* the hearer to pull envelopes it then independently
//! verifies — it never injects state directly. Under the pull transport
//! the relevant forgery is a forged-*high* summary (one claiming a head
//! above what the advertiser can actually serve): the worst it buys is one
//! wasted dial by the hearer *against itself*, bounded by the kernel's
//! per-advertiser dial-limit (`DISTRIBUTION_DIAL_DAILY_CAP`). A behind
//! peer only ever pulls for itself, so a forged summary can no longer
//! weaponise a third party into re-broadcasting (the amplification threat
//! the deleted push design carried).
//!
//! ## Why `author` is carried (not just implied by the topic)
//!
//! Symmetric with how the kernel defensively maps a misrouted
//! `Revocation`/`Publication` to a peer warning: carrying `author` in the
//! payload lets the receiver detect a summary that arrived on the wrong
//! per-author topic. Per B-12 spec §3.2.

use myrhiza_types::{AuthorPubkey, PeerPubkey};
use serde::{Deserialize, Serialize};

/// Summary advertising a peer's `last_observed_seq` for an author's
/// revocation log.
///
/// Broadcast on the per-author revocation topic derived by
/// [`derive_revocation_topic`](crate::topic::derive_revocation_topic).
/// Carried by `GossipMessage::RevocationHeads` (wire discriminant 5).
/// Unsigned — see the module docs (spec §3.6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevocationHeads {
    /// The author whose revocation log this summary describes. Carried
    /// (not just implied by the topic) so a misrouted summary is
    /// detectable. Per B-12 spec §3.2.
    pub author: AuthorPubkey,
    /// The peer advertising this summary. Used by the receiver to (a)
    /// filter loopback — a peer must ignore its own broadcast, which
    /// MemNetwork/gossip may deliver back to it — and (b) identify whom to
    /// dial for the direct-stream pull in the corrected transport (spec
    /// §13). Mirrors `HeadsSummary::signed_by_peer`. Per B-12 spec §3.2.
    pub advertiser: PeerPubkey,
    /// The highest `revocation_seq` the advertising peer has accepted
    /// for `author` (0 if it has no log for the author yet).
    pub last_observed_seq: u64,
}

/// Summary advertising a peer's `last_observed_seq` for an author's
/// publication log.
///
/// Broadcast on the per-author publication topic derived by
/// [`derive_publication_topic`](crate::topic::derive_publication_topic).
/// Carried by `GossipMessage::PublicationHeads` (wire discriminant 6).
/// Unsigned — see the module docs (spec §3.6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicationHeads {
    /// The author whose publication log this summary describes. Carried
    /// (not just implied by the topic) so a misrouted summary is
    /// detectable. Per B-12 spec §3.2.
    pub author: AuthorPubkey,
    /// The peer advertising this summary. Used for loopback filtering and
    /// (spec §13) direct-stream pull targeting. Twin of
    /// [`RevocationHeads::advertiser`]. Per B-12 spec §3.2.
    pub advertiser: PeerPubkey,
    /// The highest `publication_seq` the advertising peer has accepted
    /// for `author` (0 if it has no log for the author yet).
    pub last_observed_seq: u64,
}
