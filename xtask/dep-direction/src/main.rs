//! Binary entry point for the dep-direction check.
//!
//! Per `docs/specs/2026-05-26-b-8-sdk-design.md` §2.4 + §3.4:
//! invoked from the repo root via `cargo run -p dep-direction-check`
//! (aliased by `just dep-direction`; T8 wires it into `just ci`).
//!
//! Exit codes:
//!
//! - `0` — graph is clean. Prints `dep-direction OK` to stderr.
//! - `1` — at least one violation. Prints one diagnostic per line
//!   to stderr.
//! - `2` — `cargo metadata` itself failed (network, malformed
//!   `Cargo.toml`, etc.). Prints the underlying error to stderr.

use cargo_metadata::MetadataCommand;
use dep_direction_check::check_dep_direction;

fn main() -> std::process::ExitCode {
    let metadata = match MetadataCommand::new().exec() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: cargo metadata failed: {e}");
            return std::process::ExitCode::from(2);
        }
    };
    let violations = check_dep_direction(&metadata);
    if violations.is_empty() {
        eprintln!("dep-direction OK");
        std::process::ExitCode::SUCCESS
    } else {
        for v in &violations {
            eprintln!("{v}");
        }
        std::process::ExitCode::from(1)
    }
}
