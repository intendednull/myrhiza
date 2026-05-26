//! Binary-shellout tests for myrhiza-cli. Drives the actual built
//! binary (resolved via `env!("CARGO_BIN_EXE_myrhiza-cli")`) with
//! scripted stdin, captures stdout/stderr/exit-code via
//! `Child::wait_with_output()`.
//!
//! Per E2E-1 design §3.5. Closes the gap that `tests/e2e.rs` leaves:
//! the library-level e2e calls `myrhiza_cli::run` directly, never
//! exercising clap parsing, `main()`, or stdio handling.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::io::Write;
use std::process::{Command, Stdio};

use myrhiza_test_utils::bundle::build_signed_counter_bundle_three_components;

/// Spawn `myrhiza-cli` with the given bundle dir + author seed, pipe
/// `stdin_bytes` to stdin, collect stdout/stderr/exit-code.
fn run_cli(bundle_dir: &std::path::Path, author_seed: u64, stdin_bytes: &[u8]) -> CliOutput {
    let mut child = Command::new(env!("CARGO_BIN_EXE_myrhiza-cli"))
        .arg("--bundle")
        .arg(bundle_dir)
        .arg("--author-seed")
        .arg(author_seed.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn myrhiza-cli");
    child
        .stdin
        .as_mut()
        .expect("stdin pipe")
        .write_all(stdin_bytes)
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait_with_output");
    CliOutput {
        status: output.status.code(),
        stdout: String::from_utf8(output.stdout).expect("stdout utf-8"),
        stderr: String::from_utf8(output.stderr).expect("stderr utf-8"),
    }
}

struct CliOutput {
    status: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Covers: mvp.md §15.1 #3
///
/// Closes E2E-1 design §3.5 row 1. Binary entrypoint wires `--bundle`,
/// `--author-seed`, stdin, and stdout correctly. After scripted input
/// `inc 5\ninc 3\nquit\n`, asserts exit code 0, stdout contains the
/// progressive views `counter: 0\n`, `counter: 5\n`, `counter: 8\n`,
/// and stderr contains `final state: [0, 0, 0, 0, 0, 0, 0, 8]` (the
/// `eprintln!` at `main.rs:38` — "final state" goes to stderr).
#[test]
fn cli_binary_increment_loop_yields_final_state_via_stdout_views() {
    let (_bundle, addr) = build_signed_counter_bundle_three_components();
    let output = run_cli(&addr.bundle_dir, 0, b"inc 5\ninc 3\nquit\n");

    assert_eq!(
        output.status,
        Some(0),
        "exit code must be 0; got {:?}; stderr={:?}",
        output.status,
        output.stderr
    );
    assert!(
        output.stdout.contains("counter: 0\n"),
        "stdout must contain initial view 'counter: 0'; got: {:?}",
        output.stdout
    );
    assert!(
        output.stdout.contains("counter: 5\n"),
        "stdout must contain view after inc 5; got: {:?}",
        output.stdout
    );
    assert!(
        output.stdout.contains("counter: 8\n"),
        "stdout must contain final view 'counter: 8'; got: {:?}",
        output.stdout
    );
    assert!(
        output
            .stderr
            .contains("final state: [0, 0, 0, 0, 0, 0, 0, 8]"),
        "stderr must contain 'final state: [0, 0, 0, 0, 0, 0, 0, 8]'; got: {:?}",
        output.stderr
    );
}

/// Covers: mvp.md §15.1 #3
///
/// Closes E2E-1 design §3.5 row 2. A `--bundle` path that does not
/// exist must produce a non-zero exit code and a diagnostic on stderr
/// — not a panic, not a hang. clap accepts the path because it's just
/// a string, then `myrhiza_cli::run` hits an `open()` failure that
/// propagates as `Err(_)` through `?` in `main`, causing the binary
/// to exit non-zero.
#[test]
fn cli_binary_missing_bundle_exits_nonzero_with_diagnostic() {
    let output = run_cli(
        std::path::Path::new("/nonexistent/bundle/path-that-does-not-exist"),
        0,
        b"quit\n",
    );

    assert_ne!(
        output.status,
        Some(0),
        "exit code must be non-zero for missing bundle; got {:?}; stderr={:?}",
        output.status,
        output.stderr
    );
    assert!(
        !output.stderr.is_empty(),
        "stderr must contain a diagnostic for missing bundle; got empty stderr"
    );
}

/// Covers: mvp.md §15.1 #3
///
/// Closes E2E-1 design §3.5 row 3. Mirrors
/// `tests/e2e.rs::counter_dispatch_rejection_does_not_abort_loop`
/// through the binary entrypoint: a rejected dispatch must surface
/// `dispatch rejected:` on stdout and the loop must continue so that
/// a following valid command (`inc 1`) still applies. Exit code stays
/// 0 — a rejected dispatch is a recoverable per-line event, not a
/// fatal error.
#[test]
fn cli_binary_dispatch_rejection_does_not_abort_loop() {
    let (_bundle, addr) = build_signed_counter_bundle_three_components();
    let output = run_cli(&addr.bundle_dir, 2, b"bogus_action\ninc 1\nquit\n");

    assert_eq!(
        output.status,
        Some(0),
        "exit code must be 0 (rejected dispatch is recoverable); got {:?}; stderr={:?}",
        output.status,
        output.stderr
    );
    assert!(
        output.stdout.contains("dispatch rejected:"),
        "stdout must surface 'dispatch rejected:' for bogus action; got: {:?}",
        output.stdout
    );
    assert!(
        output
            .stderr
            .contains("final state: [0, 0, 0, 0, 0, 0, 0, 1]"),
        "stderr must contain final state [0,0,0,0,0,0,0,1] (inc 1 applied after rejection); got: {:?}",
        output.stderr
    );
}
