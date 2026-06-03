//! Direct-stream request/response surface for distribution-log backfill.
//!
//! Per B-12 spec §14.2/§14.3 (the corrected transport). This is a NEW,
//! parallel direct-stream protocol for the distribution ledger — it
//! structurally mirrors the event-DAG's B-4.4 `request_heads` protocol
//! ([`crate::request`]) but is deliberately disjoint from it: the
//! event-DAG backfill ([`crate::request::HeadsStream`] /
//! [`crate::request::HeadsResponder`] /
//! [`crate::request::RequestHandler`]) stays untouched. Coupling the two
//! ledgers onto one stream/ALPN was the runner-up and was rejected
//! (spec §14.2) — the DAG hash-chain request schema and the linear
//! distribution seq-range schema have nothing in common, and the DAG
//! backfill is wire-frozen, load-bearing code.
//!
//! A behind peer that hears an advertiser's
//! [`RevocationHeads`](myrhiza_distribution::RevocationHeads) /
//! [`PublicationHeads`](myrhiza_distribution::PublicationHeads) summary
//! with a head *above* its own dials the advertiser over
//! [`DISTRIBUTION_REQUEST_ALPN`] and pulls the missing envelopes
//! ([`DistributionEnvelope`]) from the advertiser's archive. Point-to-point
//! QUIC bypasses the Plumtree joiner→established asymmetry that defeated
//! the original gossip-push design (spec §13).

use myrhiza_distribution::{DistributionBackfillRequest, DistributionEnvelope};
use myrhiza_types::PeerPubkey;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;

/// ALPN registered against `iroh::protocol::Router` for the direct-stream
/// distribution-backfill protocol. Distinct from
/// [`crate::request::HEADS_REQUEST_ALPN`] (the event-DAG backfill) — the
/// two protocols are disjoint (spec §14.2). Follows the n0 ALPN
/// convention `<name>/<version>`.
pub const DISTRIBUTION_REQUEST_ALPN: &[u8] = b"myrhiza/distribution-request/1";

/// Bounded mpsc channel capacity for in-flight envelopes on a
/// [`DistributionStream`]. Distribution backfills are tiny (a contiguous
/// revocation range, or a single latest publication), so a small buffer
/// is ample — mirrors [`crate::request::HEADS_STREAM_CHANNEL_CAPACITY`].
pub(crate) const DISTRIBUTION_STREAM_CHANNEL_CAPACITY: usize = 32;

/// Errors surfaced through [`DistributionStream::next`].
///
/// Mirrors [`crate::request::HeadsStreamError`] — same three-variant
/// shape (transport / decode / handler), kept as a distinct type so the
/// distribution stream's item type cannot be confused with the event-DAG
/// stream's.
#[derive(Debug, Error)]
pub enum DistributionStreamError {
    /// The underlying transport failed mid-stream (QUIC reset, peer
    /// dropped, etc.). Carries a human-readable diagnostic.
    #[error("transport error: {0}")]
    Transport(String),
    /// A frame on the stream did not decode under the canonical bincode
    /// contract. Stream is terminated.
    #[error("decode failed: {0}")]
    Decode(String),
    /// The handler reported an internal error before the stream
    /// completed.
    ///
    /// **Not constructed yet.** Reserved, mirroring
    /// [`crate::request::HeadsStreamError::Handler`], for a future
    /// kernel-side handler that surfaces archive-query failures through
    /// this variant. Kept in the channel item type so a handler has a
    /// way to push errors back if a `send_err` affordance lands.
    #[allow(
        dead_code,
        reason = "reserved for kernel-side handler errors, mirroring HeadsStreamError::Handler"
    )]
    #[error("handler error: {0}")]
    Handler(String),
}

/// Receive-side stream of [`DistributionEnvelope`] items returned by
/// [`crate::Network::request_distribution`]. Polls until `None`
/// (responder closed cleanly) or the next error.
///
/// Mirrors [`crate::request::HeadsStream`].
pub struct DistributionStream {
    rx: mpsc::Receiver<Result<DistributionEnvelope, DistributionStreamError>>,
}

impl DistributionStream {
    /// Construct from an mpsc receiver. Crate-private — callers reach
    /// this via [`crate::Network::request_distribution`].
    pub(crate) fn new(
        rx: mpsc::Receiver<Result<DistributionEnvelope, DistributionStreamError>>,
    ) -> Self {
        Self { rx }
    }

    /// Receive the next envelope in the stream.
    ///
    /// - `Some(Ok(envelope))` — next envelope in the response sequence.
    /// - `Some(Err(_))` — an error occurred; the stream is terminated
    ///   and subsequent calls will return `None`.
    /// - `None` — responder closed cleanly; no further envelopes.
    pub async fn next(&mut self) -> Option<Result<DistributionEnvelope, DistributionStreamError>> {
        self.rx.recv().await
    }
}

/// Sender half of a [`DistributionStream`], used by
/// [`DistributionHandler`] implementations to write the response.
///
/// Each call to `send` pushes one [`DistributionEnvelope`] onto the
/// response stream. Drop the responder to signal "no more envelopes"
/// (clean EOF). Mirrors [`crate::request::HeadsResponder`].
pub struct DistributionResponder {
    tx: mpsc::Sender<Result<DistributionEnvelope, DistributionStreamError>>,
}

impl DistributionResponder {
    /// Construct from an mpsc sender. Crate-private — callers reach this
    /// via [`DistributionHandler::handle`].
    pub(crate) fn new(
        tx: mpsc::Sender<Result<DistributionEnvelope, DistributionStreamError>>,
    ) -> Self {
        Self { tx }
    }

    /// Push an envelope onto the response stream. Returns `Err(())` if
    /// the requester dropped the stream before consuming this envelope
    /// (in which case the handler should stop producing).
    ///
    /// # Errors
    /// Returns `Err(())` on receiver-dropped (request canceled).
    pub async fn send(&self, envelope: DistributionEnvelope) -> Result<(), ()> {
        self.tx.send(Ok(envelope)).await.map_err(|_| ())
    }
}

/// Handler invoked on the responder side when a direct-stream
/// distribution-backfill request arrives. The handler validates the
/// request (it serves the named author), reads its signed-envelope
/// archive, and pushes envelopes through [`DistributionResponder::send`].
///
/// Mirrors [`crate::request::RequestHandler`].
///
/// **Author validation**: the handler MUST verify it serves
/// `request.author` before pushing envelopes. The trait does not enforce
/// this — a handler that skips the author gate becomes a confused-deputy
/// risk, symmetric with the topic gate on [`crate::request::RequestHandler`].
#[async_trait::async_trait]
pub trait DistributionHandler: Send + Sync + 'static {
    /// Service a direct-stream distribution-backfill request from
    /// `requester`. The handler pushes response envelopes through
    /// `responder`. Returning closes the stream cleanly.
    async fn handle(
        &self,
        requester: PeerPubkey,
        request: DistributionBackfillRequest,
        responder: DistributionResponder,
    );
}

/// Convenient `Arc`'d trait object for [`DistributionHandler`].
pub type ArcDistributionHandler = Arc<dyn DistributionHandler>;
