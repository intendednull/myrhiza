//! Direct-stream request/response surface for `HeadsRequest` backfill.
//!
//! Per B-4.4 spec §3.1.

// NetError is imported for Task 3 (Network trait extension); unused until then.
#[allow(unused_imports)]
use crate::NetError;
use myrhiza_types::{DirectHeadsRequest, Event, PeerPubkey};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;

/// ALPN registered against `iroh::protocol::Router` for direct-stream
/// `HeadsRequest`. Per B-4.4 spec §2 (n0 ALPN convention `<name>/<version>`).
pub const HEADS_REQUEST_ALPN: &[u8] = b"myrhiza/heads-request/1";

/// Bounded mpsc channel capacity for in-flight events on a
/// `HeadsStream`. A small buffer here keeps memory pressure bounded
/// while still allowing burst-write throughput. 32 is sufficient for
/// the bounded-by-256 batches that [`DirectHeadsRequest::requests`]
/// targets in practice.
#[expect(dead_code, reason = "used by MemNetwork/IrohNetwork impls landing in Tasks 4-5")]
pub(crate) const HEADS_STREAM_CHANNEL_CAPACITY: usize = 32;

/// Maximum size of a single framed message (request or event). Bounds
/// memory pressure on the read side. 4 MiB is generous for events;
/// `DirectHeadsRequest` payloads are tiny in practice.
#[expect(dead_code, reason = "used by IrohNetwork framing impl landing in Task 5")]
pub(crate) const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;

/// Errors surfaced through [`HeadsStream::next`].
#[derive(Debug, Error)]
pub enum HeadsStreamError {
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
    #[error("handler error: {0}")]
    Handler(String),
}

/// Receive-side stream of [`Event`] items returned by
/// [`crate::Network::request_heads`]. Polls until `None` (responder
/// closed cleanly) or the next error.
///
/// Per B-4.4 spec §2 "`HeadsStream` shape".
pub struct HeadsStream {
    rx: mpsc::Receiver<Result<Event, HeadsStreamError>>,
}

impl HeadsStream {
    /// Construct from an mpsc receiver. Crate-private — callers reach
    /// this via [`crate::Network::request_heads`].
    #[expect(dead_code, reason = "called by Network::request_heads impls landing in Tasks 4-5")]
    pub(crate) fn new(rx: mpsc::Receiver<Result<Event, HeadsStreamError>>) -> Self {
        Self { rx }
    }

    /// Receive the next event in the stream.
    ///
    /// - `Some(Ok(event))` — next event in the response sequence.
    /// - `Some(Err(_))` — an error occurred; the stream is terminated
    ///   and subsequent calls will return `None`.
    /// - `None` — responder closed cleanly; no further events.
    pub async fn next(&mut self) -> Option<Result<Event, HeadsStreamError>> {
        self.rx.recv().await
    }
}

/// Sender half of a `HeadsStream`, used by [`RequestHandler`]
/// implementations to write the response.
///
/// Each call to `send` pushes one [`Event`] onto the response stream.
/// Drop the responder to signal "no more events" (clean EOF).
pub struct HeadsResponder {
    tx: mpsc::Sender<Result<Event, HeadsStreamError>>,
}

impl HeadsResponder {
    /// Construct from an mpsc sender. Crate-private — callers reach
    /// this via [`RequestHandler::handle`].
    #[expect(dead_code, reason = "called by RequestHandler dispatch impl landing in Tasks 4-5")]
    pub(crate) fn new(tx: mpsc::Sender<Result<Event, HeadsStreamError>>) -> Self {
        Self { tx }
    }

    /// Push an event onto the response stream. Returns `Err(())` if
    /// the requester dropped the stream before consuming this event
    /// (in which case the handler should stop producing).
    ///
    /// # Errors
    /// Returns `Err(())` on receiver-dropped (request canceled).
    pub async fn send(&self, event: Event) -> Result<(), ()> {
        self.tx.send(Ok(event)).await.map_err(|_| ())
    }
}

/// Handler invoked on the responder side when a direct-stream request
/// arrives. The handler validates the request, queries its DAG, and
/// pushes events through [`HeadsResponder::send`].
///
/// Per B-4.4 spec §2 "Handler shape".
///
/// **Topic validation**: the handler MUST verify it services
/// `request.topic` before pushing events. The trait does not
/// enforce this — handlers that skip the topic gate become a
/// confused-deputy risk.
#[async_trait::async_trait]
pub trait RequestHandler: Send + Sync + 'static {
    /// Service a direct-stream request from `requester`. The handler
    /// pushes response events through `responder`. Returning closes
    /// the stream cleanly.
    async fn handle(
        &self,
        requester: PeerPubkey,
        request: DirectHeadsRequest,
        responder: HeadsResponder,
    );
}

/// Convenient `Arc`'d trait object for [`RequestHandler`].
pub type ArcRequestHandler = Arc<dyn RequestHandler>;

// ---- Length-prefix framing helpers ----

/// Build a length-prefixed frame: `(u32 BE length, payload bytes)`.
// Used by tests in this module and by IrohNetwork framing impl landing in Task 5.
#[allow(dead_code)]
#[allow(clippy::expect_used)] // Caller is required to enforce MAX_FRAME_BYTES; panic is intentional.
pub(crate) fn build_length_prefixed_frame(payload: &[u8]) -> Vec<u8> {
    let len = u32::try_from(payload.len())
        .expect("frame payload size fits in u32 (caller bounds frames by MAX_FRAME_BYTES)");
    let mut out = Vec::with_capacity(4 + payload.len());
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(payload);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_frame_basic() {
        let frame = build_length_prefixed_frame(&[1, 2, 3]);
        assert_eq!(&frame[..4], &[0, 0, 0, 3]); // big-endian length
        assert_eq!(&frame[4..], &[1, 2, 3]);
    }

    #[test]
    fn build_frame_empty_payload() {
        let frame = build_length_prefixed_frame(&[]);
        assert_eq!(&frame[..4], &[0, 0, 0, 0]);
        assert_eq!(frame.len(), 4);
    }
}
