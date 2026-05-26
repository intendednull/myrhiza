//! Two-app coexistence acceptance tests — plan B-5.
//!
//! Closes mvp.md §15.1 criterion 4: "different state component, different
//! topic, same peer; events don't cross". Two WASM bundles (counter +
//! echo), two Runtime instances sharing one `MemBus` and the same peer
//! keypair, two distinct topics. Events authored on one runtime must
//! NOT appear in the other's state.
//!
//! Also contains a smoke test for the echo fixture mirroring the
//! existing `acceptance.rs::kernel_instantiates_and_applies_increment`.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use bincode::Options;
use myrhiza_kernel::identity::{AuthorKeypair, PeerKeypair};
use myrhiza_kernel::runtime::{AuthorCommand, PeerWarning, Runtime};
use myrhiza_kernel::{ApplyOutcome, InstallFlow, StateApplyHandle};
use myrhiza_network::{MemBus, MemNetwork};
use myrhiza_test_utils::bundle::build_signed_echo_bundle;
use myrhiza_types::{BundleHash, Event, EventHash, GenesisV1, Hlc, Topic, canonical_bincode};
use myrhiza_wasmtime_backend::WasmtimeBackend;

use myrhiza_backend::Backend;

mod helpers;

/// Wait for a `RuntimeHandle::digest_watch` to reach `expected` within
/// `timeout`. Mirrors the logic in `PeerHandle::await_digest`
/// (test-utils/src/harness.rs) but operates directly on
/// `tokio::sync::watch::Receiver<Vec<u8>>` so it can be used with the
/// raw `RuntimeHandle` the coexistence tests hold.
async fn await_runtime_digest(
    rx: &mut tokio::sync::watch::Receiver<Vec<u8>>,
    expected: &[u8],
    timeout: Duration,
) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    // Pre-wait: if an unobserved change is already present, check it first.
    if rx.has_changed().unwrap_or(false) {
        if *rx.borrow_and_update() == expected {
            return true;
        }
    } else {
        // Mark current value observed so changed() only fires on new values.
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
            Ok(Err(_)) => {
                // Sender dropped — final check.
                return *rx.borrow() == expected;
            }
            Err(_) => {
                // Poll timeout — re-check deadline at top of loop.
            }
        }
    }
}

// ============================================================================
// Test 1: smoke test for echo fixture
// ============================================================================

/// Covers: convergence.md §4.4
///
/// Smoke test for the echo-state-apply fixture, mirroring
/// `acceptance.rs::kernel_instantiates_and_applies_increment` for the
/// counter fixture. Proves:
///   - `InstallFlow::load` verifies the signed echo bundle,
///   - `WasmtimeBackend::instantiate_state_apply` compiles + links the echo WASM,
///   - `StateApplyHandle::apply` decodes the canonical `Event` envelope,
///   - genesis extracts `GenesisV1::app_payload` as initial state,
///   - non-genesis stores the event's raw payload as new state (overwrite
///     semantics).
///
/// `signature` is left zero: `handle.apply` does not verify signatures
/// (kernel verifies at insert).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn kernel_instantiates_and_applies_echo() {
    let (_bundle, addr) = build_signed_echo_bundle();

    let flow = InstallFlow::new();
    let loaded = flow.load(&addr).expect("load + verify");

    let backend = WasmtimeBackend::new().expect("backend constructs");
    let instance = backend
        .instantiate_state_apply(&loaded.component_bytes, &loaded.manifest)
        .expect("instantiate echo state-apply");
    let mut handle = StateApplyHandle::new(instance);

    let author = myrhiza_types::AuthorPubkey::from_bytes([1; 32]);

    // Build the Genesis event: GenesisV1::app_payload = b"hello".
    // The echo fixture's genesis branch extracts this and returns it as
    // the initial state.
    let initial_state = b"hello".to_vec();
    let genesis_payload = GenesisV1 {
        seed: [0x11; 32],
        founder_pubkey: author,
        app_payload: initial_state.clone(),
    };
    let genesis_payload_bytes = canonical_bincode()
        .serialize(&genesis_payload)
        .expect("encode genesis payload");
    let genesis = Event {
        author,
        seq: 1,
        prev: EventHash::ZERO,
        deps: BTreeSet::new(),
        hlc: Hlc {
            wall_ms: 0,
            logical: 0,
        },
        payload: genesis_payload_bytes,
        signature: [0; 64],
    };
    let genesis_bytes = canonical_bincode()
        .serialize(&genesis)
        .expect("encode genesis event");

    let result = handle
        .apply(&[], &genesis_bytes)
        .expect("genesis apply succeeds");
    assert!(
        matches!(result.outcome, ApplyOutcome::Accepted),
        "expected Accepted verdict for genesis, got {:?}",
        result.outcome
    );
    assert_eq!(
        result.new_state, initial_state,
        "echo fixture must return genesis app_payload as initial state"
    );

    // Non-genesis event: payload = b"world". Echo semantics: new state = payload.
    let world_payload = b"world".to_vec();
    let next_event = Event {
        author,
        seq: 2,
        prev: genesis.wire_hash(),
        deps: BTreeSet::new(),
        hlc: Hlc {
            wall_ms: 0,
            logical: 0,
        },
        payload: world_payload.clone(),
        signature: [0; 64],
    };
    let next_bytes = canonical_bincode()
        .serialize(&next_event)
        .expect("encode next event");

    let result = handle
        .apply(&result.new_state, &next_bytes)
        .expect("non-genesis apply succeeds");
    assert!(
        matches!(result.outcome, ApplyOutcome::Accepted),
        "expected Accepted verdict for non-genesis event, got {:?}",
        result.outcome
    );
    assert_eq!(
        result.new_state, world_payload,
        "echo fixture must return the event payload as new state (overwrite semantics)"
    );
}

// ============================================================================
// Test 2: two-app coexistence (criterion 4)
// ============================================================================

/// Covers: convergence.md §4.6 (topic identity)
///
/// Closes mvp.md §15.1 criterion 4: same peer, two distinct WASM bundles
/// (counter + echo), two distinct topics on the same `MemBus`. Events
/// authored on the counter runtime must NOT appear in the echo runtime's
/// state and vice versa.
///
/// Assertions:
///   - `dropped_at_apply` is empty on both runtimes (no cross-topic
///     event reached either runtime's `replay_full`),
///   - `digest_watch` on counter == `5_i64.to_be_bytes()` (genesis 0 +
///     Increment(+5)),
///   - `digest_watch` on echo == b"hello" (genesis extracted `app_payload`),
///   - the two digests differ (independent state),
///   - no `PeerWarning::SignatureInvalid` in either runtime's `peer_warnings`
///     (cross-topic delivery would surface as `DecodeFailed` or
///     `SignatureInvalid` since the event topics are keyed into the
///     `HeadsSummary` signature; absence proves isolation).
///
/// `PeerKeypair` does not impl `Clone`; `deterministic(501)` is called
/// twice — each call returns an independent keypair with the same
/// underlying key material, which is the B-4.5 Task 5 established
/// pattern for reusing a peer pubkey across two runtimes without Clone.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(
    clippy::too_many_lines,
    reason = "linear scenario test; splitting into helpers would obscure the protocol-shape assertion"
)]
async fn two_apps_coexist_no_event_crossing() {
    // --- Shared bus -----------------------------------------------------------
    let bus = MemBus::new(256);

    // --- Bundle hashes: use distinct hashes so the topics differ -------------
    // Counter and echo are different apps — they must have different bundle
    // hashes so their derived topics are guaranteed distinct even if the
    // topic-name and seed were identical (they are not, but this is also
    // defense-in-depth).
    let counter_bundle_hash = BundleHash::from_bytes([0xC0; 32]);
    let echo_bundle_hash = BundleHash::from_bytes([0xEC; 32]);

    let seed = [0xBB; 32];
    let topic_name = "main".to_string();

    let counter_topic = Topic::derive(&counter_bundle_hash, &seed, &topic_name);
    let echo_topic = Topic::derive(&echo_bundle_hash, &seed, &topic_name);
    assert_ne!(
        counter_topic, echo_topic,
        "counter and echo topics must differ (distinct app_bundle_hash)"
    );

    let cfg = helpers::fast_cfg(helpers::FAST_GOSSIP_TICK);

    // --- Author keypairs ------------------------------------------------------
    // Distinct author keys per runtime so there's no same-author cross-chain
    // confusion. SEED ALIGNMENT: the runtimes' internal AuthorKeypair seeds
    // (501 for counter, 502 for echo, passed to Runtime::start below) MUST
    // match these seeds — the genesis payloads embed kp_*_author.author as
    // `founder_pubkey`, and that pubkey must equal the pubkey of the
    // keypair the runtime uses to sign events. If these seeds drift,
    // genesis validation will fail silently. Cross-reference both seed
    // sources before editing.
    let kp_counter_author = AuthorKeypair::deterministic(501);
    let kp_echo_author = AuthorKeypair::deterministic(502);

    // --- Counter runtime ------------------------------------------------------
    // PeerKeypair doesn't impl Clone; call deterministic(501) twice to get
    // two independent keypairs with the same underlying key material.
    let peer_key_for_counter = PeerKeypair::deterministic(501);
    let peer_pubkey = peer_key_for_counter.public; // identical on both calls
    let net_counter = MemNetwork::new(bus.clone(), peer_pubkey);

    let runtime_counter = Runtime::start(
        net_counter,
        counter_topic,
        counter_bundle_hash,
        topic_name.clone(),
        helpers::counter_handle(),
        peer_key_for_counter,
        Some(AuthorKeypair::deterministic(501)), // seed MUST match kp_counter_author above.
        cfg.clone(),
        vec![],
    )
    .await
    .expect("runtime_counter start");

    // --- Echo runtime --------------------------------------------------------
    let peer_key_for_echo = PeerKeypair::deterministic(501);
    let net_echo = MemNetwork::new(bus.clone(), peer_key_for_echo.public);

    let runtime_echo = Runtime::start(
        net_echo,
        echo_topic,
        echo_bundle_hash,
        topic_name.clone(),
        helpers::echo_handle(),
        peer_key_for_echo,
        Some(AuthorKeypair::deterministic(502)), // seed MUST match kp_echo_author above.
        cfg,
        vec![],
    )
    .await
    .expect("runtime_echo start");

    // --- Author counter genesis (initial state = 0) + Increment(+5) ----------
    let counter_genesis_payload = GenesisV1 {
        seed,
        founder_pubkey: kp_counter_author.author,
        app_payload: 0_i64.to_be_bytes().to_vec(),
    };
    let (tx_cg, rx_cg) = tokio::sync::oneshot::channel();
    runtime_counter
        .author_tx
        .send(AuthorCommand::Author {
            payload: canonical_bincode()
                .serialize(&counter_genesis_payload)
                .expect("encode counter genesis"),
            deps: BTreeSet::new(),
            reply: tx_cg,
        })
        .await
        .expect("send counter genesis");
    rx_cg
        .await
        .expect("counter genesis recv")
        .expect("counter genesis ok");

    let (tx_ci, rx_ci) = tokio::sync::oneshot::channel();
    runtime_counter
        .author_tx
        .send(AuthorCommand::Author {
            payload: 5_i64.to_be_bytes().to_vec(),
            deps: BTreeSet::new(),
            reply: tx_ci,
        })
        .await
        .expect("send counter increment");
    rx_ci
        .await
        .expect("counter increment recv")
        .expect("counter increment ok");

    // --- Author echo genesis (initial state = b"hello") ----------------------
    let echo_genesis_payload = GenesisV1 {
        seed,
        founder_pubkey: kp_echo_author.author,
        app_payload: b"hello".to_vec(),
    };
    let (tx_eg, rx_eg) = tokio::sync::oneshot::channel();
    runtime_echo
        .author_tx
        .send(AuthorCommand::Author {
            payload: canonical_bincode()
                .serialize(&echo_genesis_payload)
                .expect("encode echo genesis"),
            deps: BTreeSet::new(),
            reply: tx_eg,
        })
        .await
        .expect("send echo genesis");
    rx_eg
        .await
        .expect("echo genesis recv")
        .expect("echo genesis ok");

    // --- Wait for each runtime to settle on its own expected state -----------
    let expected_counter = 5_i64.to_be_bytes().to_vec();
    let expected_echo = b"hello".to_vec();

    let mut counter_watch = runtime_counter.digest_watch.clone();
    let mut echo_watch = runtime_echo.digest_watch.clone();

    let counter_converged = await_runtime_digest(
        &mut counter_watch,
        &expected_counter,
        Duration::from_secs(5),
    )
    .await;
    let echo_converged =
        await_runtime_digest(&mut echo_watch, &expected_echo, Duration::from_secs(5)).await;

    assert!(
        counter_converged,
        "counter runtime must converge to 5_i64 state within deadline; \
         saw digest={:?}",
        runtime_counter.digest_watch.borrow().as_slice()
    );
    assert!(
        echo_converged,
        "echo runtime must converge to b\"hello\" state within deadline; \
         saw digest={:?}",
        runtime_echo.digest_watch.borrow().as_slice()
    );

    // --- Assertions -----------------------------------------------------------

    // 1. No events rejected by state-apply on either runtime (would indicate
    //    a cross-topic event reached replay_full and was incompatible).
    assert!(
        runtime_counter
            .dropped_at_apply
            .lock()
            .expect("dropped_at_apply mutex")
            .is_empty(),
        "counter runtime must have no dropped_at_apply entries (no cross-topic rejection)"
    );
    assert!(
        runtime_echo
            .dropped_at_apply
            .lock()
            .expect("dropped_at_apply mutex")
            .is_empty(),
        "echo runtime must have no dropped_at_apply entries (no cross-topic rejection)"
    );

    // 2. States differ (independent per-app state).
    assert_ne!(
        runtime_counter.digest_watch.borrow().as_slice(),
        runtime_echo.digest_watch.borrow().as_slice(),
        "counter and echo digests must differ (independent state)"
    );

    // 3. Counter state = 5.
    assert_eq!(
        runtime_counter.digest_watch.borrow().as_slice(),
        &5_i64.to_be_bytes()[..],
        "counter runtime final state must be 5"
    );

    // 4. Echo state = b"hello".
    assert_eq!(
        runtime_echo.digest_watch.borrow().as_slice(),
        &b"hello"[..],
        "echo runtime final state must be b\"hello\""
    );

    // 5. No SignatureInvalid warnings on either runtime. Cross-topic events
    //    would surface as DecodeFailed or SignatureInvalid because the
    //    HeadsSummary signature binds the topic; absence proves isolation.
    let counter_warnings = runtime_counter
        .peer_warnings
        .lock()
        .expect("peer_warnings mutex")
        .clone();
    let echo_warnings = runtime_echo
        .peer_warnings
        .lock()
        .expect("peer_warnings mutex")
        .clone();
    assert!(
        !counter_warnings
            .iter()
            .any(|w| matches!(w, PeerWarning::SignatureInvalid { .. })),
        "counter runtime must have no SignatureInvalid warnings; \
         saw warnings={counter_warnings:?}"
    );
    assert!(
        !echo_warnings
            .iter()
            .any(|w| matches!(w, PeerWarning::SignatureInvalid { .. })),
        "echo runtime must have no SignatureInvalid warnings; \
         saw warnings={echo_warnings:?}"
    );

    // --- Cleanup -------------------------------------------------------------
    let _ = runtime_counter
        .author_tx
        .send(AuthorCommand::Shutdown)
        .await;
    let _ = runtime_echo.author_tx.send(AuthorCommand::Shutdown).await;
}

// ============================================================================
// Test 3: poll + counter coexistence (criterion K4)
// ============================================================================
//
// Per spec §4.1.6 (coexistence locked) + §4.1.5 K4 row. Mirrors the
// counter+echo test above, substituting poll for echo. Same single-peer,
// two-Runtime, two-distinct-topics shape.

/// Discriminator byte for the poll `CreatePoll` variant. Mirrors the
/// constant in `tests/fixtures/poll-state-apply/src/lib.rs` and
/// `crates/kernel/tests/poll.rs`.
const POLL_DISCRIMINATOR_CREATE_POLL: u8 = 0x00;

/// Encode the `CreatePoll` body — `Vec<String>` of option labels — using
/// the SAME canonical layout the poll fixture's `decode_options`
/// consumes (hand-rolled u64-BE length-prefixed). Mirrors
/// `encode_options` in `crates/kernel/tests/poll.rs`.
fn poll_encode_options(options: &[String]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(options.len() as u64).to_be_bytes());
    for label in options {
        out.extend_from_slice(&(label.len() as u64).to_be_bytes());
        out.extend_from_slice(label.as_bytes());
    }
    out
}

/// Test-side replica of the poll fixture's `encode_state`. Mirrors
/// `encode_poll_state` in `crates/kernel/tests/poll.rs`. Used to compute
/// the golden post-genesis digest bytes the runtime must converge to.
fn poll_encode_state(
    creator: [u8; 32],
    options: &[String],
    votes: &BTreeMap<[u8; 32], u32>,
    ended: bool,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&creator);
    out.extend_from_slice(&(options.len() as u64).to_be_bytes());
    for label in options {
        out.extend_from_slice(&(label.len() as u64).to_be_bytes());
        out.extend_from_slice(label.as_bytes());
    }
    out.extend_from_slice(&(votes.len() as u64).to_be_bytes());
    for (author, opt_idx) in votes {
        out.extend_from_slice(author);
        out.extend_from_slice(&opt_idx.to_be_bytes());
    }
    out.push(u8::from(ended));
    out
}

/// Covers: spec §4.1.6 + §4.1.5 K4 row.
///
/// Same peer, two distinct WASM bundles (counter + poll), two distinct
/// topics on the same `MemBus`. Events authored on the counter runtime
/// must NOT appear in the poll runtime's state and vice versa. Mirrors
/// `two_apps_coexist_no_event_crossing` (counter + echo) above,
/// substituting poll for echo.
///
/// Assertions:
///   - `dropped_at_apply` is empty on both runtimes,
///   - each runtime's state-digest progresses independently to its own
///     expected value,
///   - the two digests differ (independent state),
///   - no `PeerWarning::SignatureInvalid` in either runtime's
///     `peer_warnings` (cross-topic delivery would surface as such
///     since the `HeadsSummary` signature binds the topic).
///
/// `PeerKeypair` does not impl `Clone`; `deterministic(501)` is called
/// twice — each call returns an independent keypair with the same
/// underlying key material, matching the B-4.5 Task 5 pattern and the
/// counter+echo test above.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(
    clippy::too_many_lines,
    reason = "linear scenario test; splitting into helpers would obscure the protocol-shape assertion"
)]
async fn poll_and_counter_coexist_no_event_crossing() {
    // --- Shared bus -----------------------------------------------------------
    let bus = MemBus::new(256);

    // --- Bundle hashes: use distinct hashes so the topics differ -------------
    // Counter and poll are different apps — they must have different bundle
    // hashes so their derived topics are guaranteed distinct.
    let counter_bundle_hash = BundleHash::from_bytes([0xC0; 32]);
    let poll_bundle_hash = BundleHash::from_bytes([0x90; 32]);

    let seed = [0xBB; 32];
    let topic_name = "main".to_string();

    let counter_topic = Topic::derive(&counter_bundle_hash, &seed, &topic_name);
    let poll_topic = Topic::derive(&poll_bundle_hash, &seed, &topic_name);
    assert_ne!(
        counter_topic, poll_topic,
        "counter and poll topics must differ (distinct app_bundle_hash)"
    );

    let cfg = fast_cfg();

    // --- Author keypairs ------------------------------------------------------
    // Distinct author keys per runtime. SEED ALIGNMENT: seeds (501 for
    // counter, 503 for poll, per the B-6 plan — echo uses 502) MUST match
    // the seeds passed to Runtime::start below; genesis payloads embed
    // `kp_*_author.author` as `founder_pubkey`, which must equal the
    // pubkey the runtime uses to sign events.
    let kp_counter_author = AuthorKeypair::deterministic(501);
    let kp_poll_author = AuthorKeypair::deterministic(503);

    // --- Counter runtime ------------------------------------------------------
    // PeerKeypair doesn't impl Clone; call deterministic(501) twice to get
    // two independent keypairs with the same underlying key material.
    let peer_key_for_counter = PeerKeypair::deterministic(501);
    let peer_pubkey = peer_key_for_counter.public; // identical on both calls
    let net_counter = MemNetwork::new(bus.clone(), peer_pubkey);

    let runtime_counter = Runtime::start(
        net_counter,
        counter_topic,
        counter_bundle_hash,
        topic_name.clone(),
        helpers::counter_handle(),
        peer_key_for_counter,
        Some(AuthorKeypair::deterministic(501)), // seed MUST match kp_counter_author above.
        cfg.clone(),
        vec![],
    )
    .await
    .expect("runtime_counter start");

    // --- Poll runtime --------------------------------------------------------
    let peer_key_for_poll = PeerKeypair::deterministic(501);
    let net_poll = MemNetwork::new(bus.clone(), peer_key_for_poll.public);

    let runtime_poll = Runtime::start(
        net_poll,
        poll_topic,
        poll_bundle_hash,
        topic_name.clone(),
        helpers::poll_handle(),
        peer_key_for_poll,
        Some(AuthorKeypair::deterministic(503)), // seed MUST match kp_poll_author above.
        cfg,
        vec![],
    )
    .await
    .expect("runtime_poll start");

    // --- Author counter genesis (initial state = 0) + Increment(+5) ----------
    let counter_genesis_payload = GenesisV1 {
        seed,
        founder_pubkey: kp_counter_author.author,
        app_payload: 0_i64.to_be_bytes().to_vec(),
    };
    let (tx_cg, rx_cg) = tokio::sync::oneshot::channel();
    runtime_counter
        .author_tx
        .send(AuthorCommand::Author {
            payload: canonical_bincode()
                .serialize(&counter_genesis_payload)
                .expect("encode counter genesis"),
            deps: BTreeSet::new(),
            reply: tx_cg,
        })
        .await
        .expect("send counter genesis");
    rx_cg
        .await
        .expect("counter genesis recv")
        .expect("counter genesis ok");

    let (tx_ci, rx_ci) = tokio::sync::oneshot::channel();
    runtime_counter
        .author_tx
        .send(AuthorCommand::Author {
            payload: 5_i64.to_be_bytes().to_vec(),
            deps: BTreeSet::new(),
            reply: tx_ci,
        })
        .await
        .expect("send counter increment");
    rx_ci
        .await
        .expect("counter increment recv")
        .expect("counter increment ok");

    // --- Author poll genesis (CreatePoll with two options) -------------------
    let poll_options = vec!["Yes".to_string(), "No".to_string()];
    let mut poll_app_payload = vec![POLL_DISCRIMINATOR_CREATE_POLL];
    poll_app_payload.extend_from_slice(&poll_encode_options(&poll_options));
    let poll_genesis_payload = GenesisV1 {
        seed,
        founder_pubkey: kp_poll_author.author,
        app_payload: poll_app_payload,
    };
    let (tx_pg, rx_pg) = tokio::sync::oneshot::channel();
    runtime_poll
        .author_tx
        .send(AuthorCommand::Author {
            payload: canonical_bincode()
                .serialize(&poll_genesis_payload)
                .expect("encode poll genesis"),
            deps: BTreeSet::new(),
            reply: tx_pg,
        })
        .await
        .expect("send poll genesis");
    rx_pg
        .await
        .expect("poll genesis recv")
        .expect("poll genesis ok");

    // --- Wait for each runtime to settle on its own expected state -----------
    let expected_counter = 5_i64.to_be_bytes().to_vec();
    let expected_poll = poll_encode_state(
        *kp_poll_author.author.as_bytes(),
        &poll_options,
        &BTreeMap::new(),
        false,
    );

    let mut counter_watch = runtime_counter.digest_watch.clone();
    let mut poll_watch = runtime_poll.digest_watch.clone();

    let counter_converged = await_runtime_digest(
        &mut counter_watch,
        &expected_counter,
        Duration::from_secs(5),
    )
    .await;
    let poll_converged =
        await_runtime_digest(&mut poll_watch, &expected_poll, Duration::from_secs(5)).await;

    assert!(
        counter_converged,
        "counter runtime must converge to 5_i64 state within deadline; \
         saw digest={:?}",
        runtime_counter.digest_watch.borrow().as_slice()
    );
    assert!(
        poll_converged,
        "poll runtime must converge to post-CreatePoll state within deadline; \
         saw digest={:?}",
        runtime_poll.digest_watch.borrow().as_slice()
    );

    // --- Assertions -----------------------------------------------------------

    // 1. No events rejected by state-apply on either runtime (would indicate
    //    a cross-topic event reached replay_full and was incompatible).
    assert!(
        runtime_counter
            .dropped_at_apply
            .lock()
            .expect("dropped_at_apply mutex")
            .is_empty(),
        "counter runtime must have no dropped_at_apply entries (no cross-topic rejection)"
    );
    assert!(
        runtime_poll
            .dropped_at_apply
            .lock()
            .expect("dropped_at_apply mutex")
            .is_empty(),
        "poll runtime must have no dropped_at_apply entries (no cross-topic rejection)"
    );

    // 2. States differ (independent per-app state).
    assert_ne!(
        runtime_counter.digest_watch.borrow().as_slice(),
        runtime_poll.digest_watch.borrow().as_slice(),
        "counter and poll digests must differ (independent state)"
    );

    // 3. Counter state = 5.
    assert_eq!(
        runtime_counter.digest_watch.borrow().as_slice(),
        &5_i64.to_be_bytes()[..],
        "counter runtime final state must be 5"
    );

    // 4. Poll state = post-CreatePoll golden bytes.
    assert_eq!(
        runtime_poll.digest_watch.borrow().as_slice(),
        expected_poll.as_slice(),
        "poll runtime final state must equal post-CreatePoll golden bytes"
    );

    // 5. No SignatureInvalid warnings on either runtime. Cross-topic events
    //    would surface as DecodeFailed or SignatureInvalid because the
    //    HeadsSummary signature binds the topic; absence proves isolation.
    let counter_warnings = runtime_counter
        .peer_warnings
        .lock()
        .expect("peer_warnings mutex")
        .clone();
    let poll_warnings = runtime_poll
        .peer_warnings
        .lock()
        .expect("peer_warnings mutex")
        .clone();
    assert!(
        !counter_warnings
            .iter()
            .any(|w| matches!(w, PeerWarning::SignatureInvalid { .. })),
        "counter runtime must have no SignatureInvalid warnings; \
         saw warnings={counter_warnings:?}"
    );
    assert!(
        !poll_warnings
            .iter()
            .any(|w| matches!(w, PeerWarning::SignatureInvalid { .. })),
        "poll runtime must have no SignatureInvalid warnings; \
         saw warnings={poll_warnings:?}"
    );

    // --- Cleanup -------------------------------------------------------------
    let _ = runtime_counter
        .author_tx
        .send(AuthorCommand::Shutdown)
        .await;
    let _ = runtime_poll.author_tx.send(AuthorCommand::Shutdown).await;
}
