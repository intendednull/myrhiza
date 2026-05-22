//! Two-app coexistence over real `IrohNetwork` — closes the iroh-realism
//! gap for mvp.md §15.1 criterion 4. Mirrors
//! `crates/kernel/tests/coexistence.rs::two_apps_coexist_no_event_crossing`
//! verbatim on identity binding (distinct `AuthorKeypair`s per app) and
//! asserts the same isolation properties through a real iroh swarm.
//!
//! Per spec §3.4. One in-process node participates in two iroh-gossip
//! swarms (counter + echo); address-discovery scope is per-process via
//! a shared `MemoryLookup`.

#![cfg(feature = "network-iroh")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeSet;
use std::time::Duration;

use bincode::Options;
use myrhiza_kernel::identity::{AuthorKeypair, PeerKeypair};
use myrhiza_kernel::pending::PendingCfg;
use myrhiza_kernel::runtime::{AuthorCommand, Runtime, RuntimeCfg};
use myrhiza_test_utils::iroh_harness::spawn_iroh_peer;
use myrhiza_types::{BundleHash, GenesisV1, Topic, canonical_bincode};

mod helpers;

fn fast_cfg() -> RuntimeCfg {
    RuntimeCfg {
        drift_interval: 1,
        drift_min_interval: Duration::from_secs(0),
        drift_daily_cap: u32::MAX,
        heads_summary_tick: Duration::from_millis(100),
        pending_cfg: PendingCfg::default(),
        broadcast_capacity: 256,
        kernel_fuel_table_version: 1,
        drift_stash_cap: 256,
        transport_error_halt_threshold: 5,
    }
}

/// Covers: mvp.md §15.1 #4
///
/// Two WASM bundles (counter + echo), two `Runtime` instances sharing
/// one iroh endpoint+gossip+router stack, two distinct topics. Events
/// authored on one runtime must NOT appear in the other's state.
///
/// Distinct author keypairs per app (501 for counter, 502 for echo)
/// per `coexistence.rs:259-260`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(
    clippy::too_many_lines,
    reason = "linear scenario test; splitting into helpers would obscure the protocol-shape assertion"
)]
async fn iroh_two_apps_coexist_no_event_crossing() {
    use iroh::address_lookup::MemoryLookup;

    let lookup = MemoryLookup::default();

    // Derive the iroh secret bytes from the same peer_seed (501) that
    // `PeerKeypair::deterministic` uses — same alignment trick as
    // `IrohHarness::spawn_peer` (T3.4). Both runtimes on this node use
    // the same peer identity (single peer, two apps). Endianness MUST
    // match `crates/kernel/src/identity/mod.rs:63` — `to_be_bytes`.
    let peer_seed: u64 = 501;
    let mut iroh_secret_bytes = [0u8; 32];
    iroh_secret_bytes[..8].copy_from_slice(&peer_seed.to_be_bytes());
    let stack = spawn_iroh_peer(&lookup, Some(iroh_secret_bytes), true).await;

    let counter_bundle_hash = BundleHash::from_bytes([0xC0; 32]);
    let echo_bundle_hash = BundleHash::from_bytes([0xEC; 32]);
    let seed = [0xBB; 32];
    let topic_name = "main".to_string();
    let counter_topic = Topic::derive(&counter_bundle_hash, &seed, &topic_name);
    let echo_topic = Topic::derive(&echo_bundle_hash, &seed, &topic_name);
    assert_ne!(counter_topic, echo_topic);

    let cfg = fast_cfg();
    let kp_counter_author = AuthorKeypair::deterministic(501);
    let kp_echo_author = AuthorKeypair::deterministic(502);

    // Single iroh peer; two Runtimes on two topics. Each gets its own
    // `IrohNetwork` clone — they share the underlying endpoint + gossip
    // + request_handler state via the #[derive(Clone)] added in T3.5.
    let net_counter = stack.network.clone();
    let net_echo = stack.network.clone();

    // `PeerKeypair` seeded to 501 to match the iroh_secret_bytes above —
    // mirrors `coexistence.rs:265-266`'s pattern of calling
    // deterministic(501) twice (`PeerKeypair` is not `Clone`).
    let peer_key_counter = PeerKeypair::deterministic(501);
    let peer_key_echo = PeerKeypair::deterministic(501);

    let runtime_counter = Runtime::start(
        net_counter,
        counter_topic,
        counter_bundle_hash,
        topic_name.clone(),
        helpers::counter_handle(),
        peer_key_counter,
        Some(AuthorKeypair::deterministic(501)),
        cfg.clone(),
        vec![], // bootstrap — same-process; no peer to dial.
    )
    .await
    .expect("runtime_counter start");

    let runtime_echo = Runtime::start(
        net_echo,
        echo_topic,
        echo_bundle_hash,
        topic_name.clone(),
        helpers::echo_handle(),
        peer_key_echo,
        Some(AuthorKeypair::deterministic(502)),
        cfg,
        vec![], // bootstrap — same-process; no peer to dial.
    )
    .await
    .expect("runtime_echo start");

    // Give the swarms a moment to settle.
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Counter genesis + increment.
    let counter_genesis = GenesisV1 {
        seed,
        founder_pubkey: kp_counter_author.author,
        app_payload: 0_i64.to_be_bytes().to_vec(),
    };
    author_blocking(
        &runtime_counter.author_tx,
        canonical_bincode()
            .serialize(&counter_genesis)
            .expect("encode counter genesis"),
    )
    .await;
    author_blocking(&runtime_counter.author_tx, 5_i64.to_be_bytes().to_vec()).await;

    // Echo genesis.
    let echo_genesis = GenesisV1 {
        seed,
        founder_pubkey: kp_echo_author.author,
        app_payload: b"hello".to_vec(),
    };
    author_blocking(
        &runtime_echo.author_tx,
        canonical_bincode()
            .serialize(&echo_genesis)
            .expect("encode echo genesis"),
    )
    .await;

    // Wait for both digests to settle (longer than `MemNetwork` variant
    // because iroh-gossip Plumtree forwarding adds latency).
    let mut rx_counter = runtime_counter.digest_watch.clone();
    let mut rx_echo = runtime_echo.digest_watch.clone();
    let counter_target = 5_i64.to_be_bytes().to_vec();
    let echo_target = b"hello".to_vec();
    assert!(
        await_digest(&mut rx_counter, &counter_target, Duration::from_secs(10)).await,
        "counter runtime must reach state {counter_target:?}; got {:?}",
        rx_counter.borrow().clone()
    );
    assert!(
        await_digest(&mut rx_echo, &echo_target, Duration::from_secs(10)).await,
        "echo runtime must reach state {:?}; got {:?}",
        echo_target,
        rx_echo.borrow().clone()
    );

    // Isolation: no cross-topic events on either side.
    let dropped_counter = runtime_counter
        .dropped_at_apply
        .lock()
        .expect("lock")
        .clone();
    let dropped_echo = runtime_echo.dropped_at_apply.lock().expect("lock").clone();
    assert!(
        dropped_counter.is_empty(),
        "counter dropped_at_apply must be empty; saw {dropped_counter:?}"
    );
    assert!(
        dropped_echo.is_empty(),
        "echo dropped_at_apply must be empty; saw {dropped_echo:?}"
    );

    let warns_counter = runtime_counter.peer_warnings.lock().expect("lock").clone();
    let warns_echo = runtime_echo.peer_warnings.lock().expect("lock").clone();
    assert!(
        !warns_counter.iter().any(|w| matches!(
            w,
            myrhiza_kernel::runtime::PeerWarning::SignatureInvalid { .. }
        )),
        "counter must not surface SignatureInvalid; saw {warns_counter:?}"
    );
    assert!(
        !warns_echo.iter().any(|w| matches!(
            w,
            myrhiza_kernel::runtime::PeerWarning::SignatureInvalid { .. }
        )),
        "echo must not surface SignatureInvalid; saw {warns_echo:?}"
    );
}

/// Helper: send an `AuthorCommand::Author` and await the reply.
async fn author_blocking(tx: &tokio::sync::mpsc::Sender<AuthorCommand>, payload: Vec<u8>) {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    tx.send(AuthorCommand::Author {
        payload,
        deps: BTreeSet::new(),
        reply: reply_tx,
    })
    .await
    .expect("send AuthorCommand");
    reply_rx.await.expect("author reply").expect("author ok");
}

/// Helper: poll a digest watch until it reports `expected` or `timeout`.
/// Mirrors `coexistence.rs::await_runtime_digest`.
async fn await_digest(
    rx: &mut tokio::sync::watch::Receiver<Vec<u8>>,
    expected: &[u8],
    timeout: Duration,
) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    if rx.has_changed().unwrap_or(false) {
        if *rx.borrow_and_update() == expected {
            return true;
        }
    } else {
        rx.mark_unchanged();
    }
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return false;
        }
        let r = tokio::time::timeout(remaining.min(Duration::from_millis(50)), rx.changed()).await;
        match r {
            Ok(Ok(())) => {
                if *rx.borrow() == expected {
                    return true;
                }
            }
            Ok(Err(_)) => return *rx.borrow() == expected,
            Err(_) => {}
        }
    }
}
