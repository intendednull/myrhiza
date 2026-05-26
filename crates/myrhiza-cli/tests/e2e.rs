//! E2E integration test for the `myrhiza-cli` harness loop.
//!
//! Drives the `view → dispatch → propose → pre-check → apply` loop with
//! scripted stdin against the three-component counter bundle. Asserts final
//! state and the pre-check ≡ apply invariant on every step.
//!
//! The three counter components are built from `examples/counter/`
//! (per docs/specs/2026-05-26-b-8-sdk-design.md §3.3 — the canonical
//! first-app demo) by the Justfile's `_build-example` recipe; this test
//! is the determinism canary for that migration (B-8 T6 acceptance),
//! since the "inc 5 inc 3 yields 8" assertion below pins counter
//! behavior across the source-of-truth move.
//!
//! Per spec §3.7 acceptance criterion: `"inc 5\ninc 3\nquit\n"` must
//! produce final state == `8_i64.to_be_bytes()`.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::io::Cursor;

use myrhiza_kernel::BundleAddress;
use myrhiza_kernel::event_builder::AuthorKeypair;
use myrhiza_kernel::state_apply::ApplyOutcome;
use myrhiza_test_utils::bundle::{
    build_signed_counter_bundle_three_components, build_signed_poll_bundle_three_components,
};

/// Counter genesis `app_payload`: 8-byte BE i64 initial value (0).
/// Counter-state-apply's genesis path stores these bytes verbatim as
/// the initial state — see `tests/fixtures/counter-state-apply/src/lib.rs`
/// genesis arm.
fn counter_genesis_app_payload() -> Vec<u8> {
    0_i64.to_be_bytes().to_vec()
}

/// Poll genesis `app_payload`: `[0x00] ++ canonical(options)` per B-6
/// spec §4.3 — `CreatePoll` discriminator followed by the canonical
/// `Vec<String>` encoding `tests/fixtures/poll-state-apply::decode_options`
/// expects (u64-BE count + per-entry u64-BE label-len + UTF-8 bytes).
fn poll_genesis_app_payload(options: &[&str]) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(0x00_u8); // DISCRIMINATOR_CREATE_POLL
    out.extend_from_slice(&(options.len() as u64).to_be_bytes());
    for label in options {
        out.extend_from_slice(&(label.len() as u64).to_be_bytes());
        out.extend_from_slice(label.as_bytes());
    }
    out
}

/// Drives the harness with `"inc 5\ninc 3\nquit\n"`.
///
/// Asserts:
/// - Final state == `8_i64.to_be_bytes()` (== `[0, 0, 0, 0, 0, 0, 0, 8]`).
/// - Exactly two steps logged (one per accepted dispatch).
/// - Pre-check verdict == apply verdict on every step (pre-check ≡ apply).
#[test]
fn counter_inc_5_inc_3_yields_final_state_8() {
    let key = AuthorKeypair::deterministic(0);
    let (_bundle, addr) = build_signed_counter_bundle_three_components();
    let BundleAddress::Disk { bundle_dir, .. } = &addr else {
        panic!("fixture builder returns Disk variant");
    };

    let input = b"inc 5\ninc 3\nquit\n".to_vec();
    let mut output: Vec<u8> = Vec::new();

    let (state, log) = myrhiza_cli::run(
        bundle_dir,
        &key,
        counter_genesis_app_payload(),
        Cursor::new(input),
        &mut output,
    )
    .expect("harness run completes without error");

    assert_eq!(
        state,
        8_i64.to_be_bytes().to_vec(),
        "final state should be BE i64 of 8; got {state:?}"
    );
    assert_eq!(
        log.len(),
        2,
        "expected two accepted dispatches, got {}",
        log.len()
    );
    for (i, entry) in log.iter().enumerate() {
        assert_eq!(
            entry.pre_check, entry.apply,
            "pre-check must equal apply on step {i} (action={:?})",
            entry.action
        );
        assert_eq!(
            entry.apply,
            ApplyOutcome::Accepted,
            "step {i} (action={:?}) must be accepted",
            entry.action
        );
    }
}

/// The stdout from the harness must include two view renders (before each
/// action) plus the final view (after "inc 3" before "quit").
///
/// Counter `view` format: `"counter: {n}\n"`. After genesis the counter
/// is 0; after inc 5 it is 5; after inc 3 it is 8.
#[test]
fn counter_stdout_shows_progressive_views() {
    let key = AuthorKeypair::deterministic(1);
    let (_bundle, addr) = build_signed_counter_bundle_three_components();
    let BundleAddress::Disk { bundle_dir, .. } = &addr else {
        panic!("fixture builder returns Disk variant");
    };

    let input = b"inc 5\ninc 3\nquit\n".to_vec();
    let mut output: Vec<u8> = Vec::new();

    myrhiza_cli::run(
        bundle_dir,
        &key,
        counter_genesis_app_payload(),
        Cursor::new(input),
        &mut output,
    )
    .expect("harness run completes");

    let text = std::str::from_utf8(&output).expect("stdout is valid UTF-8");
    assert!(
        text.contains("counter: 0\n"),
        "stdout should contain initial view 'counter: 0'; got: {text:?}"
    );
    assert!(
        text.contains("counter: 5\n"),
        "stdout should contain view after inc 5 'counter: 5'; got: {text:?}"
    );
    assert!(
        text.contains("counter: 8\n"),
        "stdout should contain final view 'counter: 8'; got: {text:?}"
    );
}

/// Rejection-path coverage: bogus action triggers `dispatch rejected`
/// without aborting the loop; the next legitimate `inc 1` still applies.
///
/// Exercises the `InteractionError::DispatchRejected` branch in
/// `myrhiza_cli::run` plus the continued-loop semantics.
#[test]
fn counter_dispatch_rejection_does_not_abort_loop() {
    let key = AuthorKeypair::deterministic(2);
    let (_bundle, addr) = build_signed_counter_bundle_three_components();
    let BundleAddress::Disk { bundle_dir, .. } = &addr else {
        panic!("fixture builder returns Disk variant");
    };

    let input = b"bogus_action\ninc 1\nquit\n".to_vec();
    let mut output: Vec<u8> = Vec::new();

    let (state, log) = myrhiza_cli::run(
        bundle_dir,
        &key,
        counter_genesis_app_payload(),
        Cursor::new(input),
        &mut output,
    )
    .expect("harness run completes; rejected dispatch is not a hard error");

    assert_eq!(
        state,
        1_i64.to_be_bytes().to_vec(),
        "rejected dispatch consumes no seq; final state should be 1"
    );
    assert_eq!(
        log.len(),
        1,
        "exactly one accepted step (the inc 1 after the rejection)"
    );
    let text = std::str::from_utf8(&output).expect("stdout is valid UTF-8");
    assert!(
        text.contains("dispatch rejected:"),
        "stdout should surface the rejection message; got: {text:?}"
    );
}

/// B-6 spec §4.1.4 "Harness contract addition (normative — owned by this spec)":
/// the harness MUST populate `peer_state` with the local `AuthorPubkey` (32
/// raw bytes) on every `view` call. The poll-interaction component surfaces
/// a per-peer "your vote: <opt> (<label>)" line when it sees a 32-byte
/// `peer_state`; this test is the regression for that contract.
///
/// Setup: a `CreatePoll{options=["Yes","No"]}` is applied as the bundle's
/// genesis via the new `genesis_app_payload` parameter — poll genesis must
/// embed the `CreatePoll` body per spec §4.3, and the v1 harness has no
/// "first dispatched action becomes genesis" mode (counter relied on the
/// same parameter-driven pattern; this slice generalises it). The driver
/// then dispatches `vote 0` to record the local author's vote against the
/// initialised poll. The interaction component's view renders
/// `your vote: 0 (Yes)` (per the §4.1.4 sample layout — index AND label)
/// once the local `AuthorPubkey` appears in the votes map.
///
/// Asserts `contains("your vote: 0")` per the plan T9 brief and spec
/// §4.1.4 — a substring match per the plan T4 risks-note (assertions
/// intentionally tolerate whitespace artifacts in the surrounding text
/// block).
#[test]
fn poll_dispatch_displays_per_peer_vote() {
    let key = AuthorKeypair::deterministic(3);
    let (_bundle, addr) = build_signed_poll_bundle_three_components();
    let BundleAddress::Disk { bundle_dir, .. } = &addr else {
        panic!("fixture builder returns Disk variant");
    };

    let input = b"vote 0\nquit\n".to_vec();
    let mut output: Vec<u8> = Vec::new();

    myrhiza_cli::run(
        bundle_dir,
        &key,
        poll_genesis_app_payload(&["Yes", "No"]),
        Cursor::new(input),
        &mut output,
    )
    .expect("harness run completes without error");

    let text = std::str::from_utf8(&output).expect("stdout is valid UTF-8");
    assert!(
        text.contains("your vote: 0"),
        "stdout must contain the per-peer 'your vote: 0' line per spec §4.1.4; \
         got: {text:?}"
    );
}
