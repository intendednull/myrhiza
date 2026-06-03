**Date:** 2026-05-29
**Status:** active
**Spec:** [B-12 §14 corrected transport](../specs/2026-05-29-b-12-stale-network-backfill-design.md) (direct-stream pull)
**Subject:** Replace B-12's gossip-push transport with direct-stream pull so late joiners catch up over real iroh-gossip — closes `distribution.md` §10.7.

# Plan B-12 (corrected transport) — direct-stream pull

## Preamble

Worktree: `/mnt/storage/projects/myrhiza/.claude/worktrees/b-10-revocation-wiring`
Branch: `feat/b-12-stale-network-backfill`.

Builds on the landed B-12 foundation (advertiser-bearing summary variants,
kernel archive, staleness surface, apply path). This plan implements spec §14:
the behind-peer **pulls** missing envelopes by dialing the advertiser over a new
direct-stream protocol (mirrors B-4.4 `request_heads`), replacing gossip-push.

**Discipline (lessons from two prior degraded runs):** every task must leave the
tree compiling under BOTH feature sets. Each task's implementer runs
`cargo check --workspace --all-targets` AND
`cargo check --workspace --all-targets --features network-iroh` and finishes only
when both are green. TDD: RED test first where a test tier applies. Never
`--no-verify`; zero clippy warnings; root-cause failures.

**WORKTREE CAVEAT:** do NOT run `just build-fixtures*` here (fails in a worktree);
the committed `tests/fixtures/built/*.wasm` satisfy kernel tests. Use cargo directly.

**Mechanism recap (spec §14.1):** advertise (kept) → behind peer hears
`remote_seq > local` → dials advertiser via `request_distribution` → advertiser
serves missing envelopes from the archive → behind peer applies via the existing
`handle_revocation`/`handle_publication` path. Push + amplification rate-limit are
removed; a per-author/advertiser dial-limit replaces them.

**Spec-coverage map:**

| Spec § | Task |
|---|---|
| §14.3 wire/types | T1, T2 |
| §14.2 new-method transport (Mem) | T2 |
| §14.2 new-method transport (iroh) | T3 |
| §14.4 serve side | T4 |
| §14.4 pull side (swap push→pull) | T5 |
| §14.4 harness wiring | T6 |
| §14.5 MemNetwork tests (pull) + dial-limit | T7 |
| §14.5 iroh tests un-ignored (closes §10.7) + wire round-trip | T8 |
| §14.5 wire-freeze, §14 status, docs | T9 |

---

## Task T1 — Distribution backfill wire types

**Spec ref:** §14.3.

**Files:** `crates/distribution/src/backfill.rs` (new) + `lib.rs` re-export.
- `pub enum DistributionLogKind { Revocation, Publication }`
- `pub struct DistributionBackfillRequest { pub author: AuthorPubkey, pub kind: DistributionLogKind, pub from_seq: u64 }` — "send envelopes with seq > from_seq".
- `pub enum DistributionEnvelope { Revocation(RevocationEvent), Publication(PublicationEvent) }`
- Derive `Clone, Debug, PartialEq, Eq, Serialize, Deserialize`. These live in `crates/distribution` (they wrap `RevocationEvent`/`PublicationEvent`); `crates/network` already depends on `crates/distribution` (B-11 inversion) so it can use them with no cycle.

**RED:** a canonical-bincode round-trip test for each type in the distribution crate.

**Verify:** `cargo test -p myrhiza-distribution`; `cargo run -p dep-direction-check`; both `cargo check` feature sets.

---

## Task T2 — Network surface + MemNetwork impl (+ iroh stub)

**Spec ref:** §14.3, §14.2 (Mem).

**Files:** `crates/network/src/request.rs` (or new `distribution_request.rs`),
`crates/network/src/lib.rs`, `crates/network/src/memory.rs`,
`crates/network/src/iroh_transport.rs` (stub only).
- `pub const DISTRIBUTION_REQUEST_ALPN: &[u8] = b"myrhiza/distribution-request/1";`
- `pub struct DistributionStream` (wraps `mpsc::Receiver<Result<DistributionEnvelope, DistributionStreamError>>`), `pub struct DistributionResponder` (tx side), `pub trait DistributionHandler { async fn handle(&self, requester: PeerPubkey, request: DistributionBackfillRequest, responder: DistributionResponder); }`, `pub type ArcDistributionHandler`, `DistributionStreamError`. Mirror `request.rs`'s `HeadsStream`/`HeadsResponder`/`RequestHandler` shapes exactly.
- `Network` trait: add `async fn request_distribution(&self, peer, DistributionBackfillRequest) -> Result<DistributionStream, NetError>;` + `fn install_distribution_handler(&self, ArcDistributionHandler);`.
- `MemNetwork`: real impl mirroring `request_heads`/`install_request_handler` (store handler in the bus; spawn handler task; channel the responder). The bus needs a `distribution_handlers` map (or reuse the pattern).
- `IrohNetwork`: **stub** both methods so the crate compiles (e.g. `request_distribution` returns `NetError::RequestFailed{reason:"unimplemented (T3)"}`; `install_distribution_handler` stores into a slot but isn't wired to a protocol yet). T3 makes it real.

**Verify:** both `cargo check` feature sets; `cargo test -p myrhiza-network` (Mem round-trip if a test is added here).

---

## Task T3 — IrohNetwork distribution direct-stream impl

**Spec ref:** §14.2 (iroh).

**Files:** `crates/network/src/iroh_transport.rs`.
- Add `DistributionRequestProtocol` implementing `iroh::protocol::ProtocolHandler` on `DISTRIBUTION_REQUEST_ALPN`, mirroring `HeadsRequestProtocol`: read length-prefixed `DistributionBackfillRequest`, dispatch to the installed `DistributionHandler`, stream `DistributionEnvelope` frames back.
- Implement `IrohNetwork::request_distribution` (mirror `request_heads`: connect on the new ALPN, write request frame, spawn a `read_distribution_frames` reader → `DistributionStream`).
- Register the protocol on the endpoint router (wherever `HEADS_REQUEST_ALPN` is registered).

**RED/Verify:** a `crates/network` iroh wire round-trip test (mirror the heads-request transport test) — peer A installs a `DistributionHandler` that serves canned envelopes; peer B `request_distribution`s and receives them. `cargo test -p myrhiza-network --features network-iroh`.

---

## Task T4 — Kernel serve side

**Spec ref:** §14.4 (serve).

**Files:** `crates/kernel/src/runtime.rs`, `start()`.
- `struct DistributionRequestCommand { requester: PeerPubkey, request: DistributionBackfillRequest, responder: DistributionResponder }`.
- `struct KernelDistributionHandler { tx: mpsc::Sender<DistributionRequestCommand>, installed_authors: Vec<AuthorPubkey> }` impl `DistributionHandler`: accept only requests whose `author` is installed (else drop → clean EOF), forward into the mailbox.
- `Runtime` gains `distribution_req_rx: mpsc::Receiver<DistributionRequestCommand>`; `start()` creates the channel, builds + `install_distribution_handler`s the handler (alongside the heads handler).
- 8th select arm: `Some(cmd) = self.distribution_req_rx.recv() => self.serve_distribution_request(cmd).await;`.
- `serve_distribution_request`: for `Revocation`, stream `revocation_archive[author]` entries with seq in `from_seq+1..=max`; for `Publication`, send `publication_latest[author]` if its seq > `from_seq`. Snapshot before `await` (the `serve_direct_heads_request` borrow discipline). Send `DistributionEnvelope`s via the responder.

**Verify:** both `cargo check` feature sets; `cargo test -p myrhiza-kernel`.

---

## Task T5 — Kernel pull side (swap push→pull)

**Spec ref:** §14.1, §14.4 (pull).

**Files:** `crates/kernel/src/runtime.rs`.
- Store a `distribution_tx: mpsc::Sender<(AuthorPubkey, GossipMessage)>` on `Runtime` (clone of the channel `subscribe_distribution_topics` builds) so pulled envelopes re-enter the apply path via `handle_distribution_message`.
- `handle_revocation_heads` / `handle_publication_heads`: **remove the push branch**. New behavior: if `remote_seq > local` (we are behind) AND the per-advertiser dial-limit admits, call `issue_distribution_backfill(heads.advertiser, author, kind, local_seq)`. (At-or-ahead → no-op.) The loopback filter already prevents acting on our own summaries.
- `issue_distribution_backfill(advertiser, author, kind, from_seq)`: `network.request_distribution(advertiser, DistributionBackfillRequest{author, kind, from_seq})`; spawn `drain_distribution_response` forwarding each `DistributionEnvelope` into `distribution_tx` as `GossipMessage::Revocation/Publication`. On request error → `PeerWarning::DirectRequestFailed`.
- Replace `distribution_push_limit` / `DISTRIBUTION_PUSH_DAILY_CAP` / `admit_distribution_push` with a **dial-limit** (`distribution_dial_limit` per author; reuse `DriftRateLimit`) bounding how often we dial for an author (defends against a forged-high-summary dial flood). Rename the constant to `DISTRIBUTION_DIAL_DAILY_CAP`.
- Optional guard: skip dialing if a pull for `(author, kind)` is already in flight (track an in-flight set) to avoid redundant concurrent dials.

**Verify:** both `cargo check` feature sets; `cargo test -p myrhiza-kernel`.

---

## Task T6 — Harness wiring

**Spec ref:** §14.4.

**Files:** `crates/test-utils/src/harness.rs`, `crates/test-utils/src/iroh_harness.rs`.
- Both `spawn_peer`s already start a `Runtime` that installs its own handlers, so the distribution handler is installed inside `Runtime::start` — confirm no extra harness wiring is needed beyond what `Runtime::start` does. If the iroh harness must register the new ALPN on the endpoint (as it does `HEADS_REQUEST_ALPN`), add it.

**Verify:** both `cargo check` feature sets; `cargo test -p myrhiza-test-utils` (+ `--features network-iroh`).

---

## Task T7 — MemNetwork acceptance tests (pull) + dial-limit

**Spec ref:** §14.5.

**Files:** `crates/kernel/tests/stale_backfill.rs`.
- Keep the outcome assertions (late-joiner range catch-up, latest-wins publication, staleness) — the mechanism is now pull; the observable results are unchanged, so the tests should pass with minimal/no change beyond removing push-specific assumptions.
- Replace `forged_low_summary_flood_is_rate_limited` with a **dial-limit** test: a peer that hears many forged-high summaries dials at most `DISTRIBUTION_DIAL_DAILY_CAP` times (observe via a counting `DistributionHandler` on an instrumented peer, or via `peer_warnings`/a request counter).
- Update the file's module doc (drop "push-on-behind"/"amplification rate-limit"; describe pull).

**Verify:** `cargo test -p myrhiza-kernel --test stale_backfill`.

---

## Task T8 — Iroh acceptance (closes §10.7) + un-ignore

**Spec ref:** §14.5, master `distribution.md` §10.7.

**Files:** `crates/kernel/tests/iroh_stale_backfill.rs`.
- Remove the `#[ignore]` attributes + the stale module-doc paragraph about gossip not working; update the doc to describe the pull mechanism.
- Adjust topology if needed: the late joiner must hear the advertiser's summary (established→joiner gossip works) then dial it (point-to-point). Bootstrap B to the anchor + C as needed so B receives C's summary; verify B catches up. Run ≥3× to confirm non-flaky; record settle timing.

**Verify:** `cargo test -p myrhiza-kernel --features network-iroh --test iroh_stale_backfill` (3×).

---

## Task T9 — Lint + matrix + docs

**Spec ref:** §14.5, §7, §8.

**Files:** docs.
- Spec: flip the §13/§14 "not yet closed" / "pending" language to landed; mark §3.1/§3.4 superseded-and-implemented; update §2 deferred (the direct-stream item is now DONE, not deferred). Update §7 surface summary (new `Network` methods, ALPN).
- README B-12 entries → reflect §10.7 **closed** via direct-stream pull; status stays `[active]` until merged or flip to `[landed]` if appropriate.
- gap-analysis: update the 2026-05-29 note → §10.7 closed.
- Full gate: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; same `--features network-iroh`; `cargo test --workspace --all-targets`; `cargo test -p myrhiza-network -p myrhiza-kernel -p myrhiza-test-utils --features network-iroh --tests`; `cargo run -p dep-direction-check`.

---

## Done criteria

- Both feature sets compile + full gate green; the two `iroh_stale_backfill.rs`
  tests pass (no longer ignored) — late joiners catch up over real iroh-gossip.
- No new gossip-push code; push + amplification rate-limit removed; dial-limit in.
- Event-DAG `request_heads` backfill untouched (additive new protocol).
- Wire-freeze: discriminants 0–6 unchanged; new request/envelope encodings pinned
  if the suite covers them.
- Docs (spec §13/§14, README, gap-analysis) reflect §10.7 closed.
