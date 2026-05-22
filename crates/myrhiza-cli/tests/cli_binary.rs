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
