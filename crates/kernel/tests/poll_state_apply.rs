//! State-tier unit tests for the poll-state-apply fixture (plan B-6).
//!
//! Per [docs/specs/2026-05-26-b-6-poll-app-design.md §4.1.5][spec] +
//! §5.1, these tests invoke `state-apply.apply(prior, event)` directly
//! via `StateApplyHandle::apply` — no `Runtime`, no MemBus, no DAG.
//! Each test instantiates a fresh wasmtime instance via
//! `helpers::poll_handle()` so the WASM bump-allocator state cannot
//! bleed across tests.
//!
//! Coverage (one `#[test]` fn per spec §4.1.5 row):
//!
//! | # | Test name | What it verifies |
//! |---|-----------|------------------|
//! | 1 | `genesis_create_poll_accepts` | CreatePoll genesis accepts; golden-bytes assertion on the canonical PollState wire format (DETERMINISM CANARY per §4.5.1 last paragraph). |
//! | 2 | `vote_records_choice` | Vote → BTreeMap insert. |
//! | 3 | `vote_replay_overwrites_prior_choice` | Last-vote-wins via re-vote. |
//! | 4 | `end_poll_by_creator_accepts` | EndPoll creator-only gate (accept on creator). |
//! | 5 | `end_poll_by_non_creator_rejects` | EndPoll creator-only gate (reject on non-creator). |
//! | 6 | `vote_after_end_poll_rejects` | Vote after EndPoll → reject. |
//! | 6b | `vote_replay_out_of_order_converges_to_lex_last` | Sort-then-apply yields identical state bytes for either arrival order — the deps-monotonicity canary per §6.4b. |
//! | 7 | `vote_out_of_range_option_rejects` | option_index >= options.len() → reject. |
//! | 8 | `non_genesis_create_poll_rejects` | CreatePoll discriminator outside genesis context → reject. |
//! | 9 | `genesis_zero_options_rejects` | CreatePoll{options=[]} → reject. |
//! | 10 | `genesis_too_many_options_rejects` | CreatePoll{options.len() > MAX_OPTIONS} → reject. |
//!
//! [spec]: ../../../docs/specs/2026-05-26-b-6-poll-app-design.md

// Pass-by-value parameters on test-side helpers (e.g.
// `make_genesis_envelope(founder: &AuthorKeypair, options: Vec<String>)`)
// match the signatures called out in plan B-6 Task T6's brief verbatim;
// clippy::needless_pass_by_value would push us toward by-reference
// rewrites that diverge from the brief's helper API.
//
// `clippy::doc_markdown` is allowed file-wide for the same reason
// `tests/peer_authority_index.rs` allows it — the prose comments name
// types and constants (BTreeMap, MAX_OPTIONS, etc.) inline; aggressive
// backtick-everything noise hurts more than it helps in test-doc prose.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::needless_pass_by_value,
    clippy::doc_markdown
)]

use std::collections::{BTreeMap, BTreeSet};

use bincode::Options;
use myrhiza_kernel::identity::AuthorKeypair;
use myrhiza_kernel::{ApplyOutcome, StateApplyHandle};
use myrhiza_types::{Event, EventHash, GenesisV1, Hlc, canonical_bincode};

mod helpers;

// ---------------------------------------------------------------------
// Test-side helpers
// ---------------------------------------------------------------------

/// Discriminator bytes per spec §4.3. Mirrors the constants in
/// `tests/fixtures/poll-state-apply/src/lib.rs` exactly.
const DISCRIMINATOR_CREATE_POLL: u8 = 0x00;
const DISCRIMINATOR_VOTE: u8 = 0x01;
const DISCRIMINATOR_END_POLL: u8 = 0x02;

/// Genesis discriminator: `seq == 1 && prior_state.is_empty()`.
/// The fixture's seed value is irrelevant to apply semantics; we use a
/// fixed cosmetic seed so the test bytes are stable.
const GENESIS_SEED: [u8; 32] = [0x11; 32];

/// Spec §4.2 bound. Mirrored locally so the "too many options" test
/// can deterministically build a payload with `MAX_OPTIONS + 1` entries.
const MAX_OPTIONS: usize = 16;

/// Install + instantiate a fresh poll-state-apply handle. Thin wrapper
/// around `helpers::poll_handle()` (Task 5 of plan B-6).
fn build_poll_apply_handle() -> StateApplyHandle {
    helpers::poll_handle()
}

/// Encode the CreatePoll body — `Vec<String>` of option labels — using
/// the SAME canonical layout the fixture's `decode_options` consumes:
///   options_len : u64 BE
///   labels[i]   : u64 BE byte-len + raw UTF-8 bytes
///
/// (This intentionally does NOT use `canonical_bincode().serialize`
/// because the fixture is `#![no_std]` + serde-free per spec §4.5.1
/// last paragraph — the fixture decodes the hand-rolled layout below,
/// not a serde-bincode `Vec<String>`.)
fn encode_options(options: &[String]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(options.len() as u64).to_be_bytes());
    for label in options {
        out.extend_from_slice(&(label.len() as u64).to_be_bytes());
        out.extend_from_slice(label.as_bytes());
    }
    out
}

/// Test-side replica of the fixture's `encode_state` (T2). Used by the
/// determinism-canary in test 1 and any other "golden bytes" assertion.
/// Layout per spec §4.5.1 + fixture comment block:
///   creator       : 32 raw bytes
///   options_len   : u64 BE
///   options[i]    : u64 BE byte-len + raw UTF-8 bytes
///   votes_len     : u64 BE (BTreeMap iterated in key-sorted order)
///   votes[i]      : 32 bytes author + u32 BE option_index
///   ended         : 1 byte (0 or 1)
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
    for (author, opt_idx) in &votes {
        out.extend_from_slice(author);
        out.extend_from_slice(&opt_idx.to_be_bytes());
    }
    out.push(u8::from(ended));
    out
}

/// Build the canonical Event-envelope bytes for a Genesis CreatePoll
/// signed by `founder`. The fixture decodes `event.author` from offset
/// 8 of the envelope (8-byte serde_bytes len + 32 pubkey bytes) and
/// extracts `founder_pubkey` from the inner `GenesisV1` envelope's
/// app_payload prefix (`0x00 ‖ encode_options(options)`).
fn make_genesis_envelope(founder: &AuthorKeypair, options: Vec<String>) -> Vec<u8> {
    let mut app_payload = vec![DISCRIMINATOR_CREATE_POLL];
    app_payload.extend_from_slice(&encode_options(&options));

    let payload = GenesisV1 {
        seed: GENESIS_SEED,
        founder_pubkey: founder.author,
        app_payload,
    };
    let payload_bytes = canonical_bincode()
        .serialize(&payload)
        .expect("encode GenesisV1");

    // Build + sign the outer Event envelope. `EventBuilder::genesis`
    // pins `founder_pubkey = builder.author`, which matches the
    // kernel's `event.author == GenesisV1.founder_pubkey` invariant
    // (per convergence.md §4.6). State-apply re-verifies this in
    // effect by reading `founder_pubkey` from the inner envelope —
    // identical to `event.author` at genesis.
    let body = Event {
        author: founder.author,
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
    let body_hash = body.hash_signed_body();
    let signature = founder.sign_body_hash(body_hash);
    let signed = Event { signature, ..body };
    canonical_bincode()
        .serialize(&signed)
        .expect("encode Event")
}

/// Build canonical Event bytes for a Vote: payload = `[0x01 || u32 BE option_index]`.
fn make_vote_envelope(
    author: &AuthorKeypair,
    seq: u64,
    prev: EventHash,
    deps: BTreeSet<EventHash>,
    option_index: u32,
) -> Vec<u8> {
    let mut payload = vec![DISCRIMINATOR_VOTE];
    payload.extend_from_slice(&option_index.to_be_bytes());
    canonical_envelope_with(author, seq, prev, deps, payload)
}

/// Build canonical Event bytes for an EndPoll: payload = `[0x02]`.
fn make_end_poll_envelope(
    author: &AuthorKeypair,
    seq: u64,
    prev: EventHash,
    deps: BTreeSet<EventHash>,
) -> Vec<u8> {
    canonical_envelope_with(author, seq, prev, deps, vec![DISCRIMINATOR_END_POLL])
}

/// Build canonical Event bytes for a non-genesis CreatePoll (used by
/// test 8, which asserts `Reject("CreatePoll: only valid as genesis")`).
/// The payload is the same `0x00 ‖ encode_options(...)` shape as the
/// inner app_payload of GenesisV1, but lives at seq >= 2 so state-apply
/// takes the non-genesis branch.
fn make_non_genesis_create_poll_envelope(
    author: &AuthorKeypair,
    seq: u64,
    prev: EventHash,
    deps: BTreeSet<EventHash>,
    options: Vec<String>,
) -> Vec<u8> {
    let mut payload = vec![DISCRIMINATOR_CREATE_POLL];
    payload.extend_from_slice(&encode_options(&options));
    canonical_envelope_with(author, seq, prev, deps, payload)
}

/// Shared canonical-envelope assembly: signs the body via
/// `AuthorKeypair::sign_body_hash` and serializes the full Event.
fn canonical_envelope_with(
    author: &AuthorKeypair,
    seq: u64,
    prev: EventHash,
    deps: BTreeSet<EventHash>,
    payload: Vec<u8>,
) -> Vec<u8> {
    let body = Event {
        author: author.author,
        seq,
        prev,
        deps,
        hlc: Hlc {
            wall_ms: 0,
            logical: 0,
        },
        payload,
        signature: [0; 64],
    };
    let body_hash = body.hash_signed_body();
    let signature = author.sign_body_hash(body_hash);
    let signed = Event { signature, ..body };
    canonical_bincode()
        .serialize(&signed)
        .expect("encode Event")
}

/// Compute the wire hash of a serialized Event envelope. Used by
/// test 6b to derive the canonical sort key on equivocating votes.
fn wire_hash(envelope: &[u8]) -> EventHash {
    EventHash::blake3(envelope)
}

/// Apply a CreatePoll genesis on a fresh handle and return the
/// resulting state bytes — the common setup for tests 2–7.
fn apply_genesis_and_unwrap(
    handle: &mut StateApplyHandle,
    founder: &AuthorKeypair,
    options: Vec<String>,
) -> Vec<u8> {
    let envelope = make_genesis_envelope(founder, options);
    let result = handle.apply(&[], &envelope).expect("genesis apply");
    assert!(
        matches!(result.outcome, ApplyOutcome::Accepted),
        "genesis must Accept; got {:?}",
        result.outcome
    );
    result.new_state
}

// ---------------------------------------------------------------------
// State-tier tests (spec §4.1.5 cases 1, 2, 3, 4, 5, 6, 6b, 7, 8, 9, 10)
// ---------------------------------------------------------------------

/// Test 1 (DETERMINISM CANARY per spec §4.5.1 last paragraph).
/// A valid CreatePoll genesis Accepts; the resulting state bytes match
/// the canonical-bincode-of-`PollState` golden bytes computed by
/// `encode_poll_state`. If a future change swaps BTreeMap for HashMap
/// or alters the encoder's byte layout, this test fails first.
#[test]
fn genesis_create_poll_accepts() {
    let mut handle = build_poll_apply_handle();
    let alice = AuthorKeypair::deterministic(1);
    let options = vec!["Yes".to_string(), "No".to_string()];

    let envelope = make_genesis_envelope(&alice, options.clone());
    let result = handle.apply(&[], &envelope).expect("genesis apply");

    assert!(
        matches!(result.outcome, ApplyOutcome::Accepted),
        "genesis CreatePoll must Accept; got {:?}",
        result.outcome
    );
    // Golden bytes: empty votes, ended=false, creator=alice, options=["Yes","No"].
    let expected = encode_poll_state(*alice.author.as_bytes(), options, BTreeMap::new(), false);
    assert_eq!(
        result.new_state, expected,
        "state-apply output must match the canonical PollState encoding bytes-exact"
    );
}

/// Test 2: a Vote after CreatePoll records the author's choice. The
/// votes BTreeMap gains a single entry keyed by the voter's pubkey.
#[test]
fn vote_records_choice() {
    let mut handle = build_poll_apply_handle();
    let alice = AuthorKeypair::deterministic(1);
    let bob = AuthorKeypair::deterministic(2);
    let options = vec!["Yes".to_string(), "No".to_string()];

    let genesis_envelope = make_genesis_envelope(&alice, options.clone());
    let prior_state = apply_genesis_and_unwrap(&mut handle, &alice, options.clone());
    let genesis_hash = wire_hash(&genesis_envelope);

    // Bob votes for option 0 ("Yes"). Bob is on his OWN per-author chain;
    // his seq=1 references genesis via `deps`, not `prev`, so the wire
    // hash topology is correct: `prev = EventHash::ZERO` (bob's chain
    // starts at seq=1), `deps = {genesis_hash}` (cross-author causal
    // anchor per spec §6.3).
    let mut deps = BTreeSet::new();
    deps.insert(genesis_hash);
    let vote_envelope = make_vote_envelope(&bob, 1, EventHash::ZERO, deps, 0);
    let result = handle
        .apply(&prior_state, &vote_envelope)
        .expect("vote apply");

    assert!(
        matches!(result.outcome, ApplyOutcome::Accepted),
        "Vote must Accept; got {:?}",
        result.outcome
    );
    let mut expected_votes = BTreeMap::new();
    expected_votes.insert(*bob.author.as_bytes(), 0u32);
    let expected = encode_poll_state(*alice.author.as_bytes(), options, expected_votes, false);
    assert_eq!(result.new_state, expected, "votes map must contain bob → 0");
}

/// Test 3: re-voting overwrites the prior choice (last-vote-wins per
/// spec §4.1.2). Bob votes for option 0, then option 1; the final
/// state's `votes[bob]` is 1.
#[test]
fn vote_replay_overwrites_prior_choice() {
    let mut handle = build_poll_apply_handle();
    let alice = AuthorKeypair::deterministic(1);
    let bob = AuthorKeypair::deterministic(2);
    let options = vec!["Yes".to_string(), "No".to_string()];

    let genesis_envelope = make_genesis_envelope(&alice, options.clone());
    let prior_state = apply_genesis_and_unwrap(&mut handle, &alice, options.clone());
    let genesis_hash = wire_hash(&genesis_envelope);

    // First vote: bob → 0.
    let mut deps_v1 = BTreeSet::new();
    deps_v1.insert(genesis_hash);
    let v1 = make_vote_envelope(&bob, 1, EventHash::ZERO, deps_v1, 0);
    let r1 = handle.apply(&prior_state, &v1).expect("v1 apply");
    assert!(matches!(r1.outcome, ApplyOutcome::Accepted));

    // Second vote on the same chain: bob → 1. seq=2, prev=wire_hash(v1).
    let v1_hash = wire_hash(&v1);
    let v2 = make_vote_envelope(&bob, 2, v1_hash, BTreeSet::new(), 1);
    let r2 = handle.apply(&r1.new_state, &v2).expect("v2 apply");
    assert!(
        matches!(r2.outcome, ApplyOutcome::Accepted),
        "re-vote must Accept; got {:?}",
        r2.outcome
    );

    let mut expected_votes = BTreeMap::new();
    expected_votes.insert(*bob.author.as_bytes(), 1u32);
    let expected = encode_poll_state(*alice.author.as_bytes(), options, expected_votes, false);
    assert_eq!(
        r2.new_state, expected,
        "last-vote-wins: votes[bob] must be 1 after re-vote"
    );
}

/// Test 4: the poll's creator may end the poll. After EndPoll, the
/// state's `ended` flag is true and no other fields change.
#[test]
fn end_poll_by_creator_accepts() {
    let mut handle = build_poll_apply_handle();
    let alice = AuthorKeypair::deterministic(1);
    let options = vec!["Yes".to_string(), "No".to_string()];

    let genesis_envelope = make_genesis_envelope(&alice, options.clone());
    let prior_state = apply_genesis_and_unwrap(&mut handle, &alice, options.clone());
    let genesis_hash = wire_hash(&genesis_envelope);

    // Alice ends her own poll. seq=2 on alice's per-author chain.
    let end_envelope = make_end_poll_envelope(&alice, 2, genesis_hash, BTreeSet::new());
    let result = handle
        .apply(&prior_state, &end_envelope)
        .expect("end poll apply");

    assert!(
        matches!(result.outcome, ApplyOutcome::Accepted),
        "EndPoll by creator must Accept; got {:?}",
        result.outcome
    );
    let expected = encode_poll_state(*alice.author.as_bytes(), options, BTreeMap::new(), true);
    assert_eq!(result.new_state, expected, "ended flag must be true");
}

/// Test 5: a non-creator EndPoll Rejects with the spec-exact reason
/// `"EndPoll: not poll creator"`. Bob (not alice) attempts to end the
/// poll; state-apply emits Reject and the state is unchanged.
#[test]
fn end_poll_by_non_creator_rejects() {
    let mut handle = build_poll_apply_handle();
    let alice = AuthorKeypair::deterministic(1);
    let bob = AuthorKeypair::deterministic(2);
    let options = vec!["Yes".to_string(), "No".to_string()];

    let genesis_envelope = make_genesis_envelope(&alice, options);
    let prior_state = apply_genesis_and_unwrap(
        &mut handle,
        &alice,
        vec!["Yes".to_string(), "No".to_string()],
    );
    let genesis_hash = wire_hash(&genesis_envelope);

    // Bob — not the creator — attempts EndPoll. Bob's chain is its own;
    // deps anchors to genesis per the spec's non-empty-deps motivation
    // (§6.3) for completeness.
    let mut deps = BTreeSet::new();
    deps.insert(genesis_hash);
    let end_envelope = make_end_poll_envelope(&bob, 1, EventHash::ZERO, deps);
    let result = handle
        .apply(&prior_state, &end_envelope)
        .expect("apply call must succeed (Reject is a valid verdict)");

    match result.outcome {
        ApplyOutcome::Rejected(reason) => {
            assert_eq!(
                reason, "EndPoll: not poll creator",
                "Reject reason must match spec §4.1.5 row 5"
            );
        }
        ApplyOutcome::Accepted => {
            panic!("non-creator EndPoll must Reject");
        }
    }
    assert!(
        result.new_state.is_empty(),
        "Reject must produce empty new_state per ApplyResult contract"
    );
}

/// Test 6: a Vote after EndPoll Rejects. Setup: alice creates poll,
/// alice ends poll; carol then attempts to vote — Reject.
#[test]
fn vote_after_end_poll_rejects() {
    let mut handle = build_poll_apply_handle();
    let alice = AuthorKeypair::deterministic(1);
    let carol = AuthorKeypair::deterministic(3);
    let options = vec!["Yes".to_string(), "No".to_string()];

    let genesis_envelope = make_genesis_envelope(&alice, options.clone());
    let state_after_genesis = apply_genesis_and_unwrap(&mut handle, &alice, options);
    let genesis_hash = wire_hash(&genesis_envelope);

    // Alice ends.
    let end_envelope = make_end_poll_envelope(&alice, 2, genesis_hash, BTreeSet::new());
    let r_end = handle
        .apply(&state_after_genesis, &end_envelope)
        .expect("end apply");
    assert!(matches!(r_end.outcome, ApplyOutcome::Accepted));

    // Carol votes after end → Reject. Carol's chain anchors to genesis
    // via deps (spec §6.3 non-empty-deps motivation).
    let mut deps = BTreeSet::new();
    deps.insert(genesis_hash);
    let vote_envelope = make_vote_envelope(&carol, 1, EventHash::ZERO, deps, 0);
    let result = handle
        .apply(&r_end.new_state, &vote_envelope)
        .expect("apply call must succeed (Reject is a valid verdict)");

    match result.outcome {
        ApplyOutcome::Rejected(reason) => {
            assert_eq!(
                reason, "Vote: poll has ended",
                "Reject reason must match spec §4.1.5 row 6"
            );
        }
        ApplyOutcome::Accepted => panic!("Vote after EndPoll must Reject"),
    }
}

/// Test 6b (deps-monotonicity canary per spec §6.4b).
///
/// Two same-author Vote events V1 and V2 with different option_index
/// values are applied in BOTH arrival orders against the same prior
/// state. To verify the convergence guarantee at the state-tier, the
/// test simulates the kernel's canonical topo-sort by sorting the two
/// events by their wire-hash byte-lex BEFORE invoking apply — i.e., it
/// applies them in canonical order regardless of the test-visible
/// arrival order. The assertion is that BOTH arrival orders, processed
/// through the same canonical sort, produce identical final state
/// bytes.
///
/// If state-apply ever introduces ordering-dependent state (e.g.,
/// HashMap iteration order, allocator-dependent encoding), this test
/// is the first canary to flip.
#[test]
fn vote_replay_out_of_order_converges_to_lex_last() {
    let alice = AuthorKeypair::deterministic(1);
    let bob = AuthorKeypair::deterministic(2);
    let options = vec!["Yes".to_string(), "No".to_string()];

    // Genesis is shared across both arrival orders.
    let genesis_envelope = make_genesis_envelope(&alice, options.clone());
    let genesis_hash = wire_hash(&genesis_envelope);

    // Build two distinct same-author Vote events. To make them lex-
    // separable (different wire_hash), give them different chain
    // positions: V1 at seq=1 with prev=ZERO, V2 at seq=2 chained to V1.
    // The state-apply itself does not inspect prev/seq for ordering —
    // those are kernel-tier concerns — but distinct envelopes are
    // required for the canonical-sort step below.
    let mut deps_v1 = BTreeSet::new();
    deps_v1.insert(genesis_hash);
    let v1 = make_vote_envelope(&bob, 1, EventHash::ZERO, deps_v1, 0);
    let v1_hash = wire_hash(&v1);
    let v2 = make_vote_envelope(&bob, 2, v1_hash, BTreeSet::new(), 1);
    let v2_hash = wire_hash(&v2);

    // Canonical order is lex by wire_hash bytes. The "lex-last" event
    // is whichever has the byte-greater hash. We pair the wire hash with
    // the option_index it carries so the lex-last winner is identifiable
    // without re-decoding the envelope.
    let (lex_first_hash, lex_last_option) = if v1_hash.as_bytes() < v2_hash.as_bytes() {
        (v1_hash, /* v2's option_index */ 1u32)
    } else {
        (v2_hash, /* v1's option_index */ 0u32)
    };

    // Arrival order A: test feeds [v1, v2]. Sort by hash → canonical
    // order → apply in canonical order on a fresh handle.
    let apply_sorted = |events: Vec<Vec<u8>>| -> Vec<u8> {
        let mut sorted = events;
        sorted.sort_by(|a, b| wire_hash(a).as_bytes().cmp(wire_hash(b).as_bytes()));
        let mut handle = build_poll_apply_handle();
        let mut state = apply_genesis_and_unwrap(
            &mut handle,
            &alice,
            vec!["Yes".to_string(), "No".to_string()],
        );
        for e in &sorted {
            let r = handle.apply(&state, e).expect("apply");
            assert!(
                matches!(r.outcome, ApplyOutcome::Accepted),
                "vote in 6b sequence must Accept; got {:?}",
                r.outcome
            );
            state = r.new_state;
        }
        state
    };

    let final_state_arrival_a = apply_sorted(vec![v1.clone(), v2.clone()]);
    let final_state_arrival_b = apply_sorted(vec![v2.clone(), v1.clone()]);

    assert_eq!(
        final_state_arrival_a, final_state_arrival_b,
        "applying the same vote SET in canonical order must yield byte-identical state regardless of arrival order"
    );

    // Cross-check the lex-last winner: the final votes[bob] equals the
    // option_index carried by the lex-last event. This is the load-
    // bearing semantic claim of the test (per the spec row's "final
    // votes[author] is the lex-last (event-hash, option_index) pair").
    let mut expected_votes = BTreeMap::new();
    expected_votes.insert(*bob.author.as_bytes(), lex_last_option);
    let expected = encode_poll_state(*alice.author.as_bytes(), options, expected_votes, false);
    assert_eq!(
        final_state_arrival_a, expected,
        "final votes[bob] must equal lex-last event's option_index"
    );
    // Sanity-check the sort key derivation: lex_first_hash is the
    // BYTE-LESSER of the two hashes by construction.
    let other_hash = if lex_first_hash == v1_hash {
        v2_hash
    } else {
        v1_hash
    };
    assert!(
        lex_first_hash.as_bytes() < other_hash.as_bytes(),
        "internal: lex_first_hash must sort byte-less than the other"
    );
}

/// Test 7: Vote with option_index >= options.len() Rejects.
#[test]
fn vote_out_of_range_option_rejects() {
    let mut handle = build_poll_apply_handle();
    let alice = AuthorKeypair::deterministic(1);
    let bob = AuthorKeypair::deterministic(2);
    let options = vec!["Yes".to_string(), "No".to_string()]; // 2 options

    let genesis_envelope = make_genesis_envelope(&alice, options.clone());
    let prior_state = apply_genesis_and_unwrap(&mut handle, &alice, options);
    let genesis_hash = wire_hash(&genesis_envelope);

    // Vote for option_index=5 (out of range — only 2 options exist).
    let mut deps = BTreeSet::new();
    deps.insert(genesis_hash);
    let vote_envelope = make_vote_envelope(&bob, 1, EventHash::ZERO, deps, 5);
    let result = handle.apply(&prior_state, &vote_envelope).expect("apply");

    match result.outcome {
        ApplyOutcome::Rejected(reason) => {
            assert_eq!(
                reason, "Vote: option_index out of range",
                "Reject reason must match spec §4.1.5 row 7"
            );
        }
        ApplyOutcome::Accepted => panic!("out-of-range option must Reject"),
    }
}

/// Test 8: a CreatePoll discriminator at `seq > 1` (i.e., outside the
/// genesis-discriminator path) Rejects with "CreatePoll: only valid as
/// genesis". This is structurally distinct from test 1's accept path —
/// the kernel CANNOT distinguish a benign seq-2 event whose payload
/// happens to start with `0x00` from a malicious "second genesis"
/// attempt; state-apply is the only authority on this rejection.
#[test]
fn non_genesis_create_poll_rejects() {
    let mut handle = build_poll_apply_handle();
    let alice = AuthorKeypair::deterministic(1);
    let options = vec!["Yes".to_string(), "No".to_string()];

    // Establish initialized state via a legitimate genesis CreatePoll.
    let genesis_envelope = make_genesis_envelope(&alice, options.clone());
    let prior_state = apply_genesis_and_unwrap(&mut handle, &alice, options);
    let genesis_hash = wire_hash(&genesis_envelope);

    // Build a CreatePoll-shaped payload at seq=2 (NOT genesis). The
    // state-apply must take the non-genesis branch (prior_state is
    // non-empty) and match the 0x00 discriminator → Reject.
    let bogus_options = vec!["Maybe".to_string()];
    let envelope = make_non_genesis_create_poll_envelope(
        &alice,
        2,
        genesis_hash,
        BTreeSet::new(),
        bogus_options,
    );
    let result = handle.apply(&prior_state, &envelope).expect("apply");

    match result.outcome {
        ApplyOutcome::Rejected(reason) => {
            assert_eq!(
                reason, "CreatePoll: only valid as genesis",
                "Reject reason must match spec §4.1.5 row 8"
            );
        }
        ApplyOutcome::Accepted => panic!("non-genesis CreatePoll must Reject"),
    }
}

/// Test 9: genesis with zero options Rejects. A poll with no options
/// is structurally meaningless; state-apply must reject.
#[test]
fn genesis_zero_options_rejects() {
    let mut handle = build_poll_apply_handle();
    let alice = AuthorKeypair::deterministic(1);

    let envelope = make_genesis_envelope(&alice, Vec::new());
    let result = handle.apply(&[], &envelope).expect("apply");

    match result.outcome {
        ApplyOutcome::Rejected(reason) => {
            assert_eq!(
                reason, "CreatePoll: must declare \u{2265}1 option",
                "Reject reason must match spec §4.1.5 row 9"
            );
        }
        ApplyOutcome::Accepted => panic!("zero-options genesis must Reject"),
    }
}

/// Test 10: genesis with options.len() > MAX_OPTIONS Rejects.
/// MAX_OPTIONS = 16 per spec §4.2.
#[test]
fn genesis_too_many_options_rejects() {
    let mut handle = build_poll_apply_handle();
    let alice = AuthorKeypair::deterministic(1);

    // Build MAX_OPTIONS + 1 = 17 options.
    let options: Vec<String> = (0..=MAX_OPTIONS).map(|i| format!("opt{i}")).collect();
    assert_eq!(options.len(), MAX_OPTIONS + 1, "must exceed MAX_OPTIONS");

    let envelope = make_genesis_envelope(&alice, options);
    let result = handle.apply(&[], &envelope).expect("apply");

    match result.outcome {
        ApplyOutcome::Rejected(reason) => {
            assert_eq!(
                reason, "CreatePoll: must declare 1..=MAX_OPTIONS",
                "Reject reason must match spec §4.1.5 row 10"
            );
        }
        ApplyOutcome::Accepted => panic!("too-many-options genesis must Reject"),
    }
}

// `EventBuilder` (`myrhiza_kernel::event_builder::EventBuilder`) is the
// production builder pinned to a single author/keypair via
// `EventBuilder::new(&keypair)`. Because the state-tier tests in this
// file frequently vary author across events on a SHARED prior_state
// (e.g., alice creates, bob votes, alice ends), the envelope assembly
// is factored through `canonical_envelope_with` so each event names its
// own signer cleanly. The production path (kernel-tier tests, runtime
// authoring) uses EventBuilder; state-tier byte-shaping bypasses it.
