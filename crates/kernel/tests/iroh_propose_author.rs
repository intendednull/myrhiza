//! Kernel-mediated authoring over real iroh-gossip — the B-13 §7 iroh
//! smoke test. The `MemNetwork` analogue lives in `propose_author.rs`; this
//! file proves the same `propose → pre-check → sign → broadcast` path
//! survives a real iroh-gossip swarm and converges a second peer.
//!
//! Topology mirrors `iroh_convergence.rs`'s proven single-originator /
//! single-receiver shape (spec §3.3 row 1): peer A is the topic
//! originator and the only author; peer B joins by bootstrapping to A and
//! converges by ingesting A's gossiped events. The one difference from
//! `iroh_convergence.rs` is *how* A authors: instead of calling
//! `PeerHandle::author(raw_payload, deps)` directly, A drives its installed
//! `counter-state-propose` component via `RuntimeHandle::propose_and_author`
//! — the kernel runs propose, pre-checks the produced payload, signs with
//! A's installed `author_key`, inserts it into the DAG, and gossips it.
//! Peer B (apply-only, read-only) never sees propose; it just applies the
//! resulting event off the wire. Both peers must converge on the same
//! state-digest.
//!
//! Why peer A is constructed via a direct `Runtime::start` (not
//! `IrohHarness::spawn_peer`): the harness `spawn_peer` passes `None` for
//! the propose handle (the universal default for non-B-13 runtimes), so the
//! author-side runtime is built here with `Some(counter_propose_handle())`,
//! exactly as `propose_author.rs::start_runtime` does at the `MemNetwork`
//! tier. Its iroh stack still comes from the shared `spawn_iroh_peer` so the
//! endpoint registers into the harness's `MemoryLookup` and peer B can dial
//! it. Peer B is a plain `harness.spawn_peer` peer (apply-only) so it reuses
//! the proven `PeerHandle::await_digest` convergence wait.
//!
//! Timing follows `iroh_convergence.rs`: a 300ms pre-author settle for the
//! Plumtree swarm to form, then a bounded `await_digest` (10s) — no fixed
//! post-author sleep; convergence is detected by the digest watch.

#![cfg(feature = "network-iroh")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeSet;
use std::time::Duration;

use bincode::Options;
use myrhiza_kernel::identity::{AuthorKeypair, PeerKeypair};
use myrhiza_kernel::runtime::{Runtime, RuntimeHandle};
use myrhiza_test_utils::iroh_harness::{IrohHarness, IrohPeerStack, spawn_iroh_peer};
use myrhiza_types::{GenesisV1, canonical_bincode};

mod helpers;

/// Recompute the iroh endpoint secret bytes for `peer_seed` using the same
/// formula `PeerKeypair::deterministic` and `IrohHarness::spawn_peer` use
/// (`bytes[..8] = peer_seed.to_be_bytes()`), so the manually-spawned peer
/// A's endpoint identity equals `PeerKeypair::deterministic(peer_seed).public`.
/// Keeping the bytes aligned is what lets peer B bootstrap to A by pubkey.
fn iroh_secret_bytes(peer_seed: u64) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&peer_seed.to_be_bytes());
    bytes
}

/// Build a well-formed counter increment intent: `[0x00] + i64_be(delta)`
/// (the `counter-state-propose` Increment vocabulary; see
/// `propose_author.rs`). Propose validates this and emits the 8-byte BE
/// delta as the event payload.
fn increment_intent(delta: i64) -> Vec<u8> {
    let mut v = vec![0x00];
    v.extend_from_slice(&delta.to_be_bytes());
    v
}

/// Spawn peer A's iroh stack and a propose-bearing `Runtime` on the harness
/// topic. Returns the retained stack (kept alive so the endpoint outlives
/// the runtime) and the `RuntimeHandle` (which exposes `propose_and_author`
/// and `digest_watch`). `peer_seed` and `author_seed` are the deterministic
/// identity seeds; the iroh secret is derived from `peer_seed` so the
/// endpoint identity matches `PeerKeypair::deterministic(peer_seed)`.
async fn spawn_propose_peer(
    harness: &IrohHarness,
    peer_seed: u64,
    author_seed: u64,
) -> (IrohPeerStack, RuntimeHandle) {
    let stack = spawn_iroh_peer(&harness.lookup, Some(iroh_secret_bytes(peer_seed)), true).await;
    let network = stack.network.clone();
    let runtime = Runtime::start(
        network,
        harness.topic,
        harness.app_bundle_hash,
        harness.topic_name.clone(),
        helpers::counter_handle(),
        PeerKeypair::deterministic(peer_seed),
        Some(AuthorKeypair::deterministic(author_seed)),
        Some(helpers::counter_propose_handle()),
        helpers::fast_cfg(helpers::FAST_GOSSIP_TICK),
        vec![],
        vec![],
    )
    .await
    .expect("Runtime::start (iroh, propose-bearing)");
    (stack, runtime)
}

/// B-13 spec §7 iroh smoke test.
///
/// Two real `Runtime`s over the iroh harness. Peer A authors genesis
/// (counter = 0) directly, then `propose_and_author(increment_intent(5))`
/// drives the real `counter-state-propose` over real iroh-gossip: the
/// kernel signs + pre-checks + applies + gossips the produced event. Peer B
/// (read-only, apply-only) ingests the gossiped genesis + increment and must
/// converge on the same state-digest (counter = 5) as peer A.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn iroh_propose_and_author_gossips_and_converges() {
    let mut harness = IrohHarness::new([0x61; 32]);

    // Peer A: propose-bearing originator (peer/author seed 1). Built
    // directly so its runtime carries the propose handle; its iroh stack
    // is retained for the test (`peer_a_stack`) so the endpoint is not torn
    // down mid-test.
    let (peer_a_stack, peer_a) = spawn_propose_peer(&harness, 1, 1).await;
    // Peer A's endpoint pubkey is peer B's bootstrap. It equals
    // `PeerKeypair::deterministic(1).public` by the `iroh_secret_bytes`
    // alignment, but reading it off the live network avoids re-deriving.
    let peer_a_pk = peer_a_stack.network.peer_pubkey();

    // Peer B: read-only apply-only receiver, bootstrapped to A so it joins
    // A's swarm and converges by ingesting gossiped events.
    let mut peer_b = harness
        .spawn_peer(
            2,
            None,
            helpers::counter_handle(),
            helpers::fast_cfg(helpers::FAST_GOSSIP_TICK),
            vec![peer_a_pk],
            vec![],
        )
        .await;

    // Let the iroh-gossip swarm form before A publishes (matches the 300ms
    // empirical window in `iroh_convergence.rs`; without it Plumtree may
    // drop the first event while B's join is still in flight).
    tokio::time::sleep(Duration::from_millis(300)).await;

    // A authors genesis (founder = A's author key, counter = 0). Genesis is
    // a raw-payload author — propose only drives the increment below.
    let kp_a = AuthorKeypair::deterministic(1);
    let genesis = GenesisV1 {
        seed: harness.seed,
        founder_pubkey: kp_a.author,
        app_payload: 0_i64.to_be_bytes().to_vec(),
    };
    let genesis_bytes = canonical_bincode()
        .serialize(&genesis)
        .expect("encode genesis payload");
    let (gtx, grx) = tokio::sync::oneshot::channel();
    peer_a
        .author_tx
        .send(myrhiza_kernel::runtime::AuthorCommand::Author {
            payload: genesis_bytes,
            deps: BTreeSet::new(),
            reply: gtx,
        })
        .await
        .expect("send genesis author");
    grx.await.expect("genesis reply").expect("genesis authored");

    // The B-13 surface under test, over real iroh: intent → propose →
    // pre-check → sign → DAG insert → gossip. The kernel runs the real
    // `counter-state-propose`, which emits the 8-byte BE delta (5).
    peer_a
        .propose_and_author(increment_intent(5))
        .await
        .expect("propose_and_author(increment 5) over iroh");

    // Both peers must converge on counter = 5. Peer A first (its own apply
    // is local), then peer B by ingesting A's gossiped genesis + increment.
    let expected_state = 5_i64.to_be_bytes().to_vec();
    assert!(
        wait_digest(&peer_a, &expected_state, Duration::from_secs(10)).await,
        "peer A must reflect the propose-authored increment (counter = 5)"
    );
    assert!(
        peer_b
            .await_digest(expected_state.clone(), Duration::from_secs(10))
            .await,
        "peer B must converge to the propose-authored state {expected_state:?} over real iroh"
    );

    let _ = peer_a
        .author_tx
        .send(myrhiza_kernel::runtime::AuthorCommand::Shutdown)
        .await;
    peer_b.shutdown().await;
}

/// Poll peer A's `digest_watch` until it equals `expected` or `timeout`
/// elapses — the originator's own apply is local, so this is a short
/// settle, not a network wait. Mirrors the condition-wait idiom of
/// `propose_author.rs::poll_until` against the watch receiver.
async fn wait_digest(rt: &RuntimeHandle, expected: &[u8], timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if *rt.digest_watch.borrow() == expected {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
