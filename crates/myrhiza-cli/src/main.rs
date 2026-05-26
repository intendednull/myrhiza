//! `myrhiza-cli` — load a signed bundle and run the interaction loop.
//!
//! Usage: `myrhiza-cli --bundle <path> [--author-seed <u64>]`
//!
//! `--bundle` points to the bundle root directory (containing
//! `manifest.bincode` and the `components/` subtree).
//! `--author-seed` is a deterministic Ed25519 seed for demo use only;
//! production deployments should pass a key via `--author-key <path>`.

use std::io::{self, BufReader};
use std::path::PathBuf;

use clap::Parser;
use myrhiza_kernel::event_builder::AuthorKeypair;

/// Myrhiza CLI: load a bundle and run the view → dispatch → propose → apply loop.
#[derive(Parser)]
#[command(name = "myrhiza-cli")]
struct Args {
    /// Path to the bundle root directory (contains manifest.bincode).
    #[arg(long)]
    bundle: PathBuf,

    /// Deterministic author keypair seed. For demo use only.
    #[arg(long, default_value_t = 0u64)]
    author_seed: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let key = AuthorKeypair::deterministic(args.author_seed);
    // v1 single-bundle CLI: the binary entry-point ships with a
    // counter-shaped genesis payload baked in (`0_i64.to_be_bytes()`).
    // Library callers (tests, future SDK examples) pick their own
    // genesis payload via the public `run` parameter — see
    // `crates/myrhiza-cli/tests/e2e.rs::poll_dispatch_displays_per_peer_vote`
    // for the poll-shaped CreatePoll genesis layout. A future
    // `--genesis-app-payload <path>` flag is the obvious extension.
    let (state, _log) = myrhiza_cli::run(
        &args.bundle,
        &key,
        0_i64.to_be_bytes().to_vec(),
        BufReader::new(io::stdin().lock()),
        io::stdout().lock(),
    )?;
    eprintln!("final state: {state:?}");
    Ok(())
}
