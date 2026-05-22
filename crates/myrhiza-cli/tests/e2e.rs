//! E2E integration test for the `myrhiza-cli` harness loop.
//!
//! Drives the `view → dispatch → propose → pre-check → apply` loop with
//! scripted stdin against the three-component counter bundle. Asserts final
//! state and the pre-check ≡ apply invariant on every step.
//!
//! Per spec §3.7 acceptance criterion: `"inc 5\ninc 3\nquit\n"` must
//! produce final state == `8_i64.to_be_bytes()`.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::io::Cursor;

use myrhiza_kernel::event_builder::AuthorKeypair;
use myrhiza_kernel::state_apply::ApplyOutcome;
use myrhiza_test_utils::bundle::build_signed_counter_bundle_three_components;

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

    let input = b"inc 5\ninc 3\nquit\n".to_vec();
    let mut output: Vec<u8> = Vec::new();

    let (state, log) = myrhiza_cli::run(&addr.bundle_dir, &key, Cursor::new(input), &mut output)
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

    let input = b"inc 5\ninc 3\nquit\n".to_vec();
    let mut output: Vec<u8> = Vec::new();

    myrhiza_cli::run(&addr.bundle_dir, &key, Cursor::new(input), &mut output)
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

    let input = b"bogus_action\ninc 1\nquit\n".to_vec();
    let mut output: Vec<u8> = Vec::new();

    let (state, log) = myrhiza_cli::run(&addr.bundle_dir, &key, Cursor::new(input), &mut output)
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
