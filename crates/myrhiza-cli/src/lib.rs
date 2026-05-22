//! Myrhiza CLI harness loop.
//!
//! Loads a signed bundle, instantiates all three component profiles
//! (state-apply, state-propose, interaction), applies genesis, then
//! loops: project a view → read an action line → dispatch → propose →
//! build a canonical envelope → pre-check → apply → repeat.
//!
//! Per spec §3.6 / §3.7. Pre-check ≡ apply assertion is exercised on
//! every dispatched action (spec §2 Choice E).

#![deny(missing_docs)]

use std::collections::BTreeSet;
use std::io::{BufRead, Write};
use std::path::Path;

use myrhiza_backend::{Backend, BackendError};
use myrhiza_kernel::{
    BundleAddress, InstallError, InstallFlow,
    event_builder::{AuthorKeypair, EventBuilder, canonical_envelope},
    interaction::{InteractionError, InteractionHandle},
    state_apply::{ApplyError, ApplyOutcome, StateApplyHandle},
    state_propose::{ProposeError, StateProposeHandle},
};
use myrhiza_types::BundleHash;
use myrhiza_wasmtime_backend::WasmtimeBackend;
use thiserror::Error;

/// Errors produced by the harness loop.
#[derive(Debug, Error)]
pub enum HarnessError {
    /// Bundle install / load step failed.
    #[error("install error: {0}")]
    Install(#[from] InstallError),
    /// Backend construction or component instantiation failed.
    #[error("backend error: {0}")]
    Backend(#[from] BackendError),
    /// Bundle declares no state-propose component.
    #[error("bundle has no state-propose component")]
    MissingPropose,
    /// Bundle declares no interaction component.
    #[error("bundle has no interaction component")]
    MissingInteraction,
    /// The genesis event was rejected by the state-apply component.
    #[error("genesis rejected: {0}")]
    GenesisRejected(String),
    /// Pre-check ≡ apply invariant violated (spec §2 Choice E).
    #[error(
        "pre-check / apply diverged on action {action:?}: \
         pre-check={pre_check}, apply={apply}"
    )]
    PreCheckApplyDiverged {
        /// The action string that triggered the divergence.
        action: String,
        /// Debug representation of the pre-check verdict.
        pre_check: String,
        /// Debug representation of the apply verdict.
        apply: String,
    },
    /// An I/O error reading from stdin or writing to stdout.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// State-apply call failed.
    #[error("apply error: {0}")]
    Apply(#[from] ApplyError),
    /// Interaction call (view) failed at the backend level.
    #[error("interaction error: {0}")]
    Interaction(#[from] InteractionError),
}

/// Per-step record emitted by the harness loop for test assertions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepLog {
    /// The action string that was dispatched.
    pub action: String,
    /// Verdict from the pre-check dry-run call.
    pub pre_check: ApplyOutcome,
    /// Verdict from the authoritative apply call.
    pub apply: ApplyOutcome,
}

/// Run the `view → dispatch → propose → pre-check → apply` loop against the
/// bundle at `bundle_path`.
///
/// `bundle_path` must be the directory containing `manifest.bincode` and the
/// `components/` subtree. `stdin` is read line-by-line for action commands;
/// `stdout` receives the interaction component's `view` output plus any
/// error messages. An empty line is skipped; `"quit"` or EOF exits the loop.
///
/// Returns `(final_state, step_log)` on clean exit.
///
/// # Errors
///
/// Returns [`HarnessError`] if the bundle fails to load, any component fails
/// to instantiate, the genesis event is rejected, an I/O error occurs, or
/// the pre-check ≡ apply invariant is violated.
// The harness loop is a single coherent unit of work: load, instantiate,
// genesis, loop. Splitting it into sub-functions would scatter the
// borrow relationships across multiple call sites without clarity gain.
#[allow(clippy::too_many_lines)]
pub fn run<R: BufRead, W: Write>(
    bundle_path: &Path,
    author_key: &AuthorKeypair,
    mut stdin: R,
    mut stdout: W,
) -> Result<(Vec<u8>, Vec<StepLog>), HarnessError> {
    // 1. Load + verify bundle.
    let flow = InstallFlow::new();
    let addr = BundleAddress {
        bundle_dir: bundle_path.to_path_buf(),
        manifest_path: "manifest.bincode".into(),
    };
    let bundle = flow.load(&addr)?;

    // 2. Construct backend + instantiate all three profiles.
    let backend = WasmtimeBackend::new()?;

    let mut apply_handle = StateApplyHandle::new(
        backend.instantiate_state_apply(&bundle.component_bytes, &bundle.manifest)?,
    );

    let propose_bytes = bundle
        .state_propose_bytes
        .ok_or(HarnessError::MissingPropose)?;
    let mut propose_handle = StateProposeHandle::new(
        backend.instantiate_state_propose(&propose_bytes, &bundle.manifest)?,
    );

    let interaction_bytes = bundle
        .interaction_bytes
        .ok_or(HarnessError::MissingInteraction)?;
    let mut interaction_handle = InteractionHandle::new(
        backend.instantiate_interaction(&interaction_bytes, &bundle.manifest)?,
    );

    // 3. Apply genesis: counter starts at 0.
    //    Genesis payload: seed=[0;32], app_payload=0_i64 BE (8 bytes).
    let builder = EventBuilder::new(author_key);
    // BundleHash is a placeholder for v1 single-peer harness (the harness
    // does not fetch a real on-chain bundle hash; the genesis EventBuilder
    // arg is informational-only for now).
    let bundle_hash = BundleHash::from_bytes([0u8; 32]);
    let genesis_event = builder.genesis(
        &bundle_hash,
        [0u8; 32],
        "counter",
        0_i64.to_be_bytes().to_vec(),
    );
    let genesis_envelope = canonical_envelope(&genesis_event);
    let genesis_result = apply_handle.apply(&[], &genesis_envelope)?;
    if !matches!(genesis_result.outcome, ApplyOutcome::Accepted) {
        let msg = match genesis_result.outcome {
            ApplyOutcome::Rejected(m) => m,
            ApplyOutcome::Accepted => unreachable!(),
        };
        return Err(HarnessError::GenesisRejected(msg));
    }
    let mut state = genesis_result.new_state;
    let peer_state: Vec<u8> = Vec::new();
    let mut last_event = genesis_event;
    let mut step_log: Vec<StepLog> = Vec::new();

    // 4. Main loop.
    loop {
        let view = interaction_handle.view(&state, &peer_state)?;
        stdout.write_all(&view)?;
        stdout.flush()?;

        let mut line = String::new();
        let n = stdin.read_line(&mut line)?;
        if n == 0 {
            // EOF
            break;
        }
        let action = line.trim().to_string();
        if action == "quit" {
            break;
        }
        if action.is_empty() {
            continue;
        }

        // Dispatch action → intent bytes.
        let intent = match interaction_handle.dispatch(&action) {
            Ok(i) => i,
            Err(InteractionError::DispatchRejected(msg)) => {
                writeln!(stdout, "dispatch rejected: {msg}")?;
                continue;
            }
            Err(InteractionError::Backend(e)) => return Err(HarnessError::Backend(e)),
        };

        // Propose intent → payload bytes.
        let payload = match propose_handle.propose(&state, &intent) {
            Ok(p) => p,
            Err(ProposeError::Rejected(msg)) => {
                writeln!(stdout, "propose rejected: {msg}")?;
                continue;
            }
            Err(ProposeError::Backend(e)) => return Err(HarnessError::Backend(e)),
        };

        // Build a canonical signed event envelope.
        let next_event = builder.next(&last_event, BTreeSet::new(), payload);
        let next_envelope = canonical_envelope(&next_event);

        // Pre-check dry-run (spec §2 Choice E).
        let pre_check_result = apply_handle.pre_check(&state, &next_envelope)?;

        // Authoritative apply.
        let apply_result = apply_handle.apply(&state, &next_envelope)?;

        // Pre-check ≡ apply assertion — divergence is a correctness bug,
        // never relaxed (spec §2 Choice E).
        if pre_check_result.outcome != apply_result.outcome {
            return Err(HarnessError::PreCheckApplyDiverged {
                action: action.clone(),
                pre_check: format!("{:?}", pre_check_result.outcome),
                apply: format!("{:?}", apply_result.outcome),
            });
        }

        step_log.push(StepLog {
            action: action.clone(),
            pre_check: pre_check_result.outcome.clone(),
            apply: apply_result.outcome.clone(),
        });

        match apply_result.outcome {
            ApplyOutcome::Accepted => {
                state = apply_result.new_state;
                last_event = next_event;
            }
            ApplyOutcome::Rejected(ref msg) => {
                writeln!(stdout, "apply rejected: {msg}")?;
            }
        }
    }

    Ok((state, step_log))
}
