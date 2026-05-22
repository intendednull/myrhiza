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
    let (state, _log) = myrhiza_cli::run(
        &args.bundle,
        &key,
        BufReader::new(io::stdin().lock()),
        io::stdout().lock(),
    )?;
    eprintln!("final state: {state:?}");
    Ok(())
}
