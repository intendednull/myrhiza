//! Kernel-tier acceptance tests for plan B-6 (Poll app).
//!
//! Per [docs/specs/2026-05-26-b-6-poll-app-design.md §4.1.5][spec] +
//! §5.2 + §6.3, these tests drive the poll-state-apply fixture through
//! the full kernel path (Runtime + MemBus + author/pre-check/apply +
//! replay_full + dropped_at_apply diagnostics). State-tier coverage
//! (direct `state-apply.apply` invocations) lives in
//! `poll_state_apply.rs`; this file picks up at the kernel boundary.
//!
//! Coverage (one `#[tokio::test]` per spec §4.1.5 K-row):
//!
//! | # | Test name | What it verifies |
//! |---|-----------|------------------|
//! | K1 | `poll_e2e_single_peer_full_lifecycle` | full CreatePoll → Vote → ReVote → EndPoll on one peer; final state-digest byte-asserted against `encode_poll_state` golden bytes. |
//! | K2 | `poll_multi_author_voting` | three authors (alice/bob/carol) on one bus. Voters declare `deps = {genesis_hash}` (NOT empty) — the LOAD-BEARING exercise of the non-empty-deps code path in T1's decoder per spec §6.3. All three peers converge to the same digest. |
//! | K3 | `poll_unauthorized_end_poll_rejected_by_authority` | bob's hand-injected EndPoll on alice's poll lands in alice's `dropped_at_apply` with `Reject("EndPoll: not poll creator")`. |
//!
//! K1–K3 use the shared `helpers::fast_cfg(helpers::FAST_GOSSIP_TICK)`
//! from B-MINI-2 (parameterized RuntimeCfg builder, FAST_GOSSIP_TICK =
//! 100 ms for prompt cross-peer state delivery in convergence-shaped tests).
//!
//! [spec]: ../../../docs/specs/2026-05-26-b-6-poll-app-design.md

// `clippy::doc_markdown` is allowed file-wide for the same reason
// `tests/poll_state_apply.rs` allows it — the prose comments name types
// and constants (BTreeMap, EndPoll, etc.) inline; aggressive
// backtick-everything noise hurts more than it helps in test-doc prose.
//
// `clippy::needless_pass_by_value` matches the precedent set by the
// state-tier `tests/poll_state_apply.rs`: helper signatures
// (e.g. `encode_poll_state(creator, options: Vec<String>, votes: BTreeMap<...>, ended)`)
// are kept owned-by-value because every call site builds them fresh and
// rewriting to take references would obscure the byte-shape assertions
// the tests are designed to surface.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::doc_markdown,
    clippy::needless_pass_by_value
)]

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use bincode::Options;
use myrhiza_kernel::identity::AuthorKeypair;
use myrhiza_network::{GossipMessage, MemNetwork, Network};
use myrhiza_test_utils::InProcessHarness;
use myrhiza_types::{Event, EventHash, GenesisV1, Hlc, canonical_bincode};

mod helpers;

// ---------------------------------------------------------------------
// Shared kernel-tier config + test-side helpers
// ---------------------------------------------------------------------

/// Discriminator bytes per spec §4.3. Mirrors the constants in
/// `tests/fixtures/poll-state-apply/src/lib.rs` exactly.
const DISCRIMINATOR_CREATE_POLL: u8 = 0x00;
const DISCRIMINATOR_VOTE: u8 = 0x01;
const DISCRIMINATOR_END_POLL: u8 = 0x02;

/// Encode the CreatePoll body — `Vec<String>` of option labels — using
/// the SAME canonical layout the fixture's `decode_options` consumes
/// (hand-rolled u64-BE length-prefixed). See state-tier test helpers
/// in `poll_state_apply.rs` for the rationale (fixture is no_std +
/// serde-free).
fn encode_options(options: &[String]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(options.len() as u64).to_be_bytes());
    for label in options {
        out.extend_from_slice(&(label.len() as u64).to_be_bytes());
        out.extend_from_slice(label.as_bytes());
    }
    out
}

/// Test-side replica of the fixture's `encode_state` (T2). Golden
/// state-bytes builder per spec §4.5.1 / fixture comment block.
fn encode_poll_state(
    creator: [u8; 32],
    options: Vec<String>,
    votes: BTreeMap<[u8; 32], u32>,
    ended: bool,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&creator);
    out.extend_from_slice(&(options.len() as u64).to_be_bytes());
    for label in &options {
        out.extend_from_slice(&(label.len() as u64).to_be_bytes());
        out.extend_from_slice(label.as_bytes());
    }
    out.extend_from_slice(&(votes.len() as u64).to_be_bytes());
    // BTreeMap iterates sorted-by-key — load-bearing per spec §6.1.
    for (author, opt_idx) in &votes {
        out.extend_from_slice(author);
        out.extend_from_slice(&opt_idx.to_be_bytes());
    }
    out.push(u8::from(ended));
    out
}

/// Build the `GenesisV1`-wrapped CreatePoll payload bytes that the
/// kernel's `Runtime::author` accepts as the founder's first event.
/// `Runtime::author` itself wraps this into the outer `Event` envelope.
fn create_poll_genesis_payload(
    seed: [u8; 32],
    founder: &AuthorKeypair,
    options: &[String],
) -> Vec<u8> {
    let mut app_payload = vec![DISCRIMINATOR_CREATE_POLL];
    app_payload.extend_from_slice(&encode_options(options));

    let payload = GenesisV1 {
        seed,
        founder_pubkey: founder.author,
        app_payload,
    };
    canonical_bincode()
        .serialize(&payload)
        .expect("encode GenesisV1")
}

/// Build a Vote payload — discriminator + u32 BE option_index. The
/// kernel wraps this into the outer Event envelope on the author path.
fn vote_payload(option_index: u32) -> Vec<u8> {
    let mut p = vec![DISCRIMINATOR_VOTE];
    p.extend_from_slice(&option_index.to_be_bytes());
    p
}

/// Build an EndPoll payload — just the 1-byte discriminator.
fn end_poll_payload() -> Vec<u8> {
    vec![DISCRIMINATOR_END_POLL]
}

// ---------------------------------------------------------------------
// K1: full single-peer lifecycle
// ---------------------------------------------------------------------

/// Spec §4.1.5 K1 / §5.2.
///
/// One peer (alice). Authoring sequence: CreatePoll genesis →
/// Vote(option=0) → ReVote(option=1) → EndPoll. Each step rides the
/// full `Runtime::author` path: state-propose payload bytes (here
/// constructed test-side; state-propose's role is to validate the
/// intent shape, not to reshape bytes — the kernel does not call
/// state-propose) → pre-check (state-apply in dry-run) → apply →
/// replay → digest publish.
///
/// Assertion: the final `digest_watch` bytes equal the canonical
/// `encode_poll_state` output for `{creator=alice, options=["Yes","No"],
/// votes={alice→1}, ended=true}`. This is the kernel-tier counterpart
/// to state-tier test 1's golden-bytes assertion — same byte layout,
/// driven end-to-end through the kernel rather than the bare WASM
/// handle.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn poll_e2e_single_peer_full_lifecycle() {
    let harness = InProcessHarness::new(256, [0x11; 32]);
    let cfg = helpers::fast_cfg(helpers::FAST_GOSSIP_TICK);
    let mut alice = harness
        .spawn_peer(1, Some(1), helpers::poll_handle(), cfg, vec![])
        .await;
    let kp_alice = AuthorKeypair::deterministic(1);

    let options = vec!["Yes".to_string(), "No".to_string()];

    // Step 1: CreatePoll genesis. Founder = alice. Initial state =
    // PollState{creator=alice, options, votes={}, ended=false}.
    let genesis_payload = create_poll_genesis_payload(harness.seed, &kp_alice, &options);
    alice
        .author(genesis_payload, BTreeSet::new())
        .await
        .expect("alice authors CreatePoll genesis");

    // Wait for the genesis state to settle before authoring the next
    // event — otherwise the second author call may race the
    // post-genesis digest publish. The digest at this point is the
    // golden bytes for `{creator=alice, options, votes={}, ended=false}`.
    let after_genesis = encode_poll_state(
        *kp_alice.author.as_bytes(),
        options.clone(),
        BTreeMap::new(),
        false,
    );
    assert!(
        alice
            .await_digest(after_genesis, Duration::from_secs(5))
            .await,
        "alice's runtime must publish post-genesis digest"
    );

    // Step 2: alice Votes for option 0. Empty deps — alice's chain
    // (seq=2 chained to seq=1 via prev) carries the causal anchor;
    // K1 explicitly does NOT exercise the non-empty-deps code path
    // (that's K2's job per §6.3).
    alice
        .author(vote_payload(0), BTreeSet::new())
        .await
        .expect("alice Votes(0)");

    let after_vote0 = {
        let mut votes = BTreeMap::new();
        votes.insert(*kp_alice.author.as_bytes(), 0u32);
        encode_poll_state(*kp_alice.author.as_bytes(), options.clone(), votes, false)
    };
    assert!(
        alice
            .await_digest(after_vote0, Duration::from_secs(5))
            .await,
        "alice's digest must reflect Vote(0)"
    );

    // Step 3: alice ReVotes for option 1 — last-vote-wins per spec
    // §4.1.2 / state-tier test 3.
    alice
        .author(vote_payload(1), BTreeSet::new())
        .await
        .expect("alice ReVotes(1)");

    let after_revote = {
        let mut votes = BTreeMap::new();
        votes.insert(*kp_alice.author.as_bytes(), 1u32);
        encode_poll_state(*kp_alice.author.as_bytes(), options.clone(), votes, false)
    };
    assert!(
        alice
            .await_digest(after_revote, Duration::from_secs(5))
            .await,
        "alice's digest must reflect ReVote(1) — last-vote-wins overwrites prior choice"
    );

    // Step 4: alice EndPoll. After this, `ended=true` and any future
    // Vote on this state would Reject (state-tier test 6 covers that).
    alice
        .author(end_poll_payload(), BTreeSet::new())
        .await
        .expect("alice EndPoll");

    let final_state = {
        let mut votes = BTreeMap::new();
        votes.insert(*kp_alice.author.as_bytes(), 1u32);
        encode_poll_state(*kp_alice.author.as_bytes(), options.clone(), votes, true)
    };
    assert!(
        alice
            .await_digest(final_state.clone(), Duration::from_secs(5))
            .await,
        "final digest must equal canonical PollState bytes \
         for {{creator=alice, options=[\"Yes\",\"No\"], votes={{alice→1}}, ended=true}}; \
         saw {:?}",
        alice.current_digest()
    );

    assert!(
        alice.dropped_at_apply().is_empty(),
        "K1 happy-path lifecycle must not drop any events at apply; \
         saw dropped_at_apply={:?}",
        alice.dropped_at_apply()
    );

    alice.shutdown().await;
}

// ---------------------------------------------------------------------
// K2: multi-author voting with non-empty deps (load-bearing per §6.3)
// ---------------------------------------------------------------------

/// Spec §4.1.5 K2 / §5.2 / §6.3.
///
/// Three authors share one in-process bus. Alice creates the poll;
/// bob and carol each vote once with **`deps = {alice's genesis event
/// hash}`** — NOT empty. This is the LOAD-BEARING exercise of the
/// non-empty-deps code path in T1's state-apply decoder (per spec §6.3:
/// "poll is the first fixture that tolerates non-empty deps"). If the
/// fixture's byte-offset decoder fails to skip past the deps array
/// dynamically, this test surfaces it as either `PreCheckRejected` on
/// the voter side or a `dropped_at_apply` entry on the receiver side.
///
/// After alice also votes (no deps — she chains via prev on her own
/// chain), all three peers must converge to the same digest.
///
/// Assertions:
///   - alice's vote authors successfully on her runtime,
///   - bob's vote authors successfully on his runtime with
///     `deps={genesis_hash}`,
///   - carol's vote authors successfully on her runtime with
///     `deps={genesis_hash}`,
///   - all three peers' `digest_watch` converges to the same canonical
///     bytes: `{creator=alice, options=["Yes","No"],
///     votes={alice→0, bob→0, carol→1}, ended=false}`,
///   - none of the three runtimes has any `dropped_at_apply` entries
///     (would indicate a state-apply rejection of a well-formed
///     non-empty-deps event — the bug §6.3 is designed to catch).
///
/// Vote choices (alice→0, bob→0, carol→1) are deterministic and
/// asymmetric: bob+alice both pick option 0 ("Yes"), carol picks
/// option 1 ("No"). This makes the encoded-state bytes
/// distinguishable from each individual voter's contribution and
/// surfaces any BTreeMap iteration-order regression — see test 6.1
/// of the spec.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(
    clippy::too_many_lines,
    reason = "linear three-author convergence scenario; splitting into helpers would obscure the deps-flow assertion"
)]
async fn poll_multi_author_voting() {
    let harness = InProcessHarness::new(256, [0x22; 32]);
    let cfg = helpers::fast_cfg(helpers::FAST_GOSSIP_TICK);

    let mut alice = harness
        .spawn_peer(1, Some(1), helpers::poll_handle(), cfg.clone(), vec![])
        .await;
    let mut bob = harness
        .spawn_peer(2, Some(2), helpers::poll_handle(), cfg.clone(), vec![])
        .await;
    let mut carol = harness
        .spawn_peer(3, Some(3), helpers::poll_handle(), cfg, vec![])
        .await;

    let kp_alice = AuthorKeypair::deterministic(1);
    let kp_bob = AuthorKeypair::deterministic(2);
    let kp_carol = AuthorKeypair::deterministic(3);

    let options = vec!["Yes".to_string(), "No".to_string()];

    // Step 1: alice CreatePoll genesis. Capture the returned EventHash
    // — this IS the genesis_hash that voters' Votes will declare as
    // their `deps`. Per spec §6.3, this is the load-bearing non-empty
    // deps exercise: without `deps={genesis_hash}`, T1's decoder is
    // never asked to skip past a deps array.
    let genesis_payload = create_poll_genesis_payload(harness.seed, &kp_alice, &options);
    let genesis_hash = alice
        .author(genesis_payload, BTreeSet::new())
        .await
        .expect("alice authors CreatePoll genesis");

    // Wait for bob + carol to ingest the genesis before they author
    // their votes. Their state-propose courtesy check + the kernel's
    // pre-check both require prior_state to be the initialized
    // PollState; if they authored before genesis arrived their
    // prior_state would still be empty and the kernel would treat the
    // event as the genesis attempt of a fresh chain (rejected by
    // state-apply's discriminator branch).
    let post_genesis = encode_poll_state(
        *kp_alice.author.as_bytes(),
        options.clone(),
        BTreeMap::new(),
        false,
    );
    assert!(
        bob.await_digest(post_genesis.clone(), Duration::from_secs(5))
            .await,
        "bob must ingest alice's genesis before authoring his vote"
    );
    assert!(
        carol
            .await_digest(post_genesis, Duration::from_secs(5))
            .await,
        "carol must ingest alice's genesis before authoring her vote"
    );

    // Step 2: bob Votes for option 0. **Non-empty deps** — anchors
    // bob's first event in his chain to alice's genesis. This is the
    // §6.3 load-bearing assertion: the decoder must dynamically skip
    // past the deps array (8-byte len + N × 40 bytes) rather than
    // hard-coding the payload offset at a compile-time constant.
    let mut bob_deps = BTreeSet::new();
    bob_deps.insert(genesis_hash);
    bob.author(vote_payload(0), bob_deps)
        .await
        .expect("bob Votes(0) with deps={genesis_hash}");

    // Step 3: carol Votes for option 1 with the same anchor. Asymmetric
    // option choice so the resulting state encoding differs from a
    // "two-voters-same-option" tally.
    let mut carol_deps = BTreeSet::new();
    carol_deps.insert(genesis_hash);
    carol
        .author(vote_payload(1), carol_deps)
        .await
        .expect("carol Votes(1) with deps={genesis_hash}");

    // Step 4: alice also votes — option 0. Empty deps; alice's seq=2
    // chains to her own seq=1 (the genesis) via prev. This is purely
    // for completeness — to give the final tally three entries —
    // and is NOT the §6.3 exercise (alice's vote does not declare
    // non-empty deps).
    alice
        .author(vote_payload(0), BTreeSet::new())
        .await
        .expect("alice Votes(0)");

    // Expected final state: alice creator, options frozen at genesis,
    // votes = {alice→0, bob→0, carol→1}, ended=false. BTreeMap
    // iteration is sorted by 32-byte author key — the canonical
    // encoding is stable across all three peers if and only if their
    // `votes` BTreeMaps converge to the same set.
    let final_state = {
        let mut votes = BTreeMap::new();
        votes.insert(*kp_alice.author.as_bytes(), 0u32);
        votes.insert(*kp_bob.author.as_bytes(), 0u32);
        votes.insert(*kp_carol.author.as_bytes(), 1u32);
        encode_poll_state(*kp_alice.author.as_bytes(), options.clone(), votes, false)
    };

    // All three peers must converge — the spec's "first multi-author
    // fixture with non-empty deps" convergence proof.
    assert!(
        alice
            .await_digest(final_state.clone(), Duration::from_secs(10))
            .await,
        "alice must converge to three-vote tally; saw {:?}",
        alice.current_digest()
    );
    assert!(
        bob.await_digest(final_state.clone(), Duration::from_secs(10))
            .await,
        "bob must converge to three-vote tally; saw {:?}",
        bob.current_digest()
    );
    assert!(
        carol
            .await_digest(final_state, Duration::from_secs(10))
            .await,
        "carol must converge to three-vote tally; saw {:?}",
        carol.current_digest()
    );

    // No state-apply rejections on any peer — non-empty deps must be
    // tolerated. A failure here surfaces a T1 decoder bug (per the
    // brief's "If K2 fails with 'payload extends past event bytes' or
    // 'non-empty deps not supported', that's a T1 decoder bug").
    assert!(
        alice.dropped_at_apply().is_empty(),
        "alice must record no apply-time rejections; saw {:?}",
        alice.dropped_at_apply()
    );
    assert!(
        bob.dropped_at_apply().is_empty(),
        "bob must record no apply-time rejections; saw {:?}",
        bob.dropped_at_apply()
    );
    assert!(
        carol.dropped_at_apply().is_empty(),
        "carol must record no apply-time rejections; saw {:?}",
        carol.dropped_at_apply()
    );

    alice.shutdown().await;
    bob.shutdown().await;
    carol.shutdown().await;
}

// ---------------------------------------------------------------------
// K3: unauthorized EndPoll → Reject at apply → dropped_at_apply entry
// ---------------------------------------------------------------------

/// Spec §4.1.5 K3 / §5.2 / §4.4.
///
/// Setup: alice creates a poll on her runtime. Bob hand-constructs a
/// signed EndPoll Event (non-founder seq=1, `deps={genesis_hash}`) and
/// publishes it directly onto the shared bus — bypassing
/// `Runtime::author`'s pre-check (which would otherwise reject it on
/// bob's side, since bob's local state-apply would also see alice as
/// creator). Alice's runtime ingests bob's event via gossip, the DAG
/// accepts it (sig + chain valid; DAG never invokes state-apply), then
/// `replay_full` calls `apply` and the fixture returns
/// `Reject("EndPoll: not poll creator")` per state-tier test 5.
///
/// The mirror of `convergence::dropped_at_apply_records_rejected_events`
/// pattern: the originator-pre-check path is unusable here (it would
/// short-circuit before broadcast), so we hand-inject onto the bus.
///
/// Assertions:
///   - alice's `dropped_at_apply` contains exactly one entry,
///   - keyed by bob's EndPoll event wire hash,
///   - reason equals `"EndPoll: not poll creator"` (the spec-exact
///     string in state-tier test 5).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn poll_unauthorized_end_poll_rejected_by_authority() {
    let harness = InProcessHarness::new(256, [0x33; 32]);
    let cfg = helpers::fast_cfg(helpers::FAST_GOSSIP_TICK);
    let mut alice = harness
        .spawn_peer(1, Some(1), helpers::poll_handle(), cfg, vec![])
        .await;
    let kp_alice = AuthorKeypair::deterministic(1);
    let kp_bob = AuthorKeypair::deterministic(2);

    let options = vec!["Yes".to_string(), "No".to_string()];

    // Step 1: alice CreatePoll. Capture the genesis EventHash so we
    // can anchor bob's hostile EndPoll to it (non-founder seq=1 with
    // explicit deps — the DAG validates the deps edge against the
    // existing event).
    let genesis_payload = create_poll_genesis_payload(harness.seed, &kp_alice, &options);
    let genesis_hash = alice
        .author(genesis_payload, BTreeSet::new())
        .await
        .expect("alice authors CreatePoll genesis");

    // Wait for alice's runtime to publish the post-genesis digest so
    // the test surface is past the construction-default (empty) and
    // any subsequent digest change is observable as the bob-EndPoll
    // path's effect.
    let after_genesis = encode_poll_state(
        *kp_alice.author.as_bytes(),
        options.clone(),
        BTreeMap::new(),
        false,
    );
    assert!(
        alice
            .await_digest(after_genesis.clone(), Duration::from_secs(5))
            .await,
        "alice's runtime must publish post-genesis digest"
    );

    // Step 2: hand-construct bob's EndPoll. `Runtime::author` on bob's
    // OWN runtime would pre-check and reject before broadcast (the
    // dropped_at_apply pattern requires bypassing pre-check) — so we
    // build the Event manually mirroring the
    // `perf_carryovers::replay_fallback_when_drain_loop_inserts_multiple`
    // non-founder-seq=1-with-deps pattern.
    //
    // Bob's event: author=bob, seq=1, prev=ZERO, deps={genesis_hash},
    // payload=[0x02] (EndPoll discriminator + empty body). This is
    // exactly what `make_end_poll_envelope` in `poll_state_apply.rs`
    // would emit at the state-tier — built once here directly because
    // `EventBuilder` doesn't expose a non-founder-seq=1-with-deps
    // helper (its `genesis` always uses empty deps).
    let bob_end_poll = {
        let mut deps = BTreeSet::new();
        deps.insert(genesis_hash);
        let body = Event {
            author: kp_bob.author,
            seq: 1,
            prev: EventHash::ZERO,
            deps,
            hlc: Hlc {
                wall_ms: 0,
                logical: 0,
            },
            payload: end_poll_payload(),
            signature: [0; 64],
        };
        let body_hash = body.hash_signed_body();
        let signature = kp_bob.sign_body_hash(body_hash);
        Event { signature, ..body }
    };
    let bob_event_hash = bob_end_poll.wire_hash();

    // Publish bob's EndPoll directly onto the bus (a separate
    // `MemNetwork` instance with a throwaway peer pubkey, mirroring
    // the `dropped_at_apply_records_rejected_events` pattern from
    // `convergence.rs`). Alice's runtime receives it via her recv
    // loop, the DAG accepts (sig + chain valid), `replay_full`
    // invokes state-apply, which returns `Reject("EndPoll: not poll
    // creator")` — landing the entry in `dropped_at_apply`.
    let net_pub = MemNetwork::new(
        harness.bus.clone(),
        myrhiza_types::PeerPubkey::from_bytes([0xC3; 32]),
    );
    net_pub
        .publish(harness.topic, GossipMessage::Event(bob_end_poll))
        .await
        .expect("publish bob's hostile EndPoll");

    // Poll alice's dropped_at_apply until it records the rejection
    // (matches the `dropped_at_apply_records_rejected_events`
    // polling pattern — avoids racing the recv loop with a fixed
    // sleep). `await_digest` is not suitable here because the digest
    // does NOT change on apply-time rejection — the event is
    // accepted into the DAG but skipped during state materialization
    // per spec §4.4 / §14 edge-case 8.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let map = loop {
        let snapshot = alice.dropped_at_apply();
        if !snapshot.is_empty() {
            break snapshot;
        }
        if std::time::Instant::now() >= deadline {
            break snapshot;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    };

    assert_eq!(
        map.len(),
        1,
        "alice's dropped_at_apply must contain exactly one entry — \
         bob's hostile EndPoll; saw map={map:?}"
    );
    let reason = map
        .get(&bob_event_hash)
        .expect("dropped_at_apply must be keyed by bob's EndPoll wire_hash");
    assert_eq!(
        reason, "EndPoll: not poll creator",
        "reject reason must match spec §4.1.5 row 5 / fixture's exact string"
    );

    // alice's digest must still equal the post-genesis state — bob's
    // EndPoll was rejected at apply, so it does NOT commit to state.
    // Spec §4.4: "the event is *not* removed from the DAG ... only
    // the state-materialization step skips it".
    assert_eq!(
        alice.current_digest(),
        after_genesis,
        "alice's digest must remain at post-genesis state (bob's rejected EndPoll does not commit)"
    );

    alice.shutdown().await;
}
