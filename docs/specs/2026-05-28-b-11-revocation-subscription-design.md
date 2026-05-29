**Date:** 2026-05-28
**Status:** landed
**Parent:** [docs/specs/2026-05-09-myrhiza-master-design/README.md](2026-05-09-myrhiza-master-design/README.md)
**Predecessor:** [docs/specs/2026-05-26-b-10-bundle-distribution-design.md](2026-05-26-b-10-bundle-distribution-design.md)
**Subject:** Plan B-11 — Kernel Runtime subscription wiring for revocation + publication topics

# Plan B-11 design — Revocation/publication subscription wiring

## 1. Goal

B-10 shipped the `crates/distribution/` **pure tier** — `RevocationLog`/`PublicationLog` state machines, per-author topic derivation (`derive_revocation_topic`/`derive_publication_topic`), the `dispatch::verify_*` gossip-edge verifiers, and the `signed_envelope` machinery — but **never connected it to the kernel**. `crates/kernel/src` has zero references to any of it (verified 2026-05-28). The revocation mechanism is therefore *mechanically complete but operationally inert*: an author can publish a `RevocationEvent`, and no peer will ever act on it.

This is the exact half-done state CLAUDE.md warns against ("shipping revocation machinery that no peer can act on"). The [MVP gap analysis](../reports/2026-05-21-mvp-gap-analysis.md) marks item 14 (bundle distribution + signing) as ✅, but its own footnote concedes: *"Kernel-tier runtime subscription wiring for revocation is deferred to a follow-up (see B-10 spec §10)."* B-11 is that follow-up.

B-11 wires the distribution pure tier into the kernel `Runtime` so that:

1. On install of a bundle from author X, the `Runtime` **auto-subscribes** to X's revocation topic *and* publication topic (per [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.7: *"Every peer that has ever installed an app or module signed by this author auto-subscribes to the author's revocation topic on install."*).
2. The `Runtime` select loop **dispatches** inbound revocation/publication gossip: `dispatch::verify_*` at the edge → `RevocationLog::apply`/`PublicationLog::apply` (seq-monotonicity + `MAX_REVOCATION_JUMP` flood cap) → surface on success.
3. The `Runtime` holds **per-author** `RevocationLog`/`PublicationLog` state.
4. The kernel **surfaces** `RevocationApplied`/`PublicationAnnounced` outward through the established `RuntimeHandle` poll-log pattern (the codebase's "kernel UI surface"), so an embedder can drive the [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.5 step 7 uninstall prompt.
5. A kernel-tier acceptance test (`crates/kernel/tests/iroh_revocation.rs`) proves end-to-end propagation over real iroh-gossip, closing B-10 spec §6.4.

After B-11, the B-10 revocation/publication design is live, not just resident.

## 2. Scope

### In v1 (this slice)

- **Two `GossipMessage` variants** — append `Revocation(RevocationEvent)` and `Publication(PublicationEvent)` to `myrhiza_network::GossipMessage` at discriminants 3 and 4 (existing `Event=0`/`HeadsSummary=1`/`Drift=2` untouched — wire-freeze preserved). `crates/network` gains an unconditional dep on `crates/distribution` (pure tier, no iroh). Two new `wire_freeze.rs` discriminant-pinning tests.
- **`Runtime::start(... , installed_authors: Vec<AuthorPubkey>)`** — new parameter. For each author, subscribe to both derived topics at spawn, spawn a drainer task per subscription forwarding into a shared `mpsc` channel.
- **Sixth select-loop arm** — polls the shared distribution `mpsc::Receiver<(AuthorPubkey, GossipMessage)>`, dispatches to `handle_revocation`/`handle_publication`.
- **Per-author log state** — `revocation_logs: BTreeMap<AuthorPubkey, RevocationLog>` + `publication_logs: BTreeMap<AuthorPubkey, PublicationLog>` on `Runtime`.
- **Outward surface** — `RevocationApplied`/`PublicationAnnounced` types + `Arc<Mutex<Vec<…>>>` poll-logs on `Runtime` and `RuntimeHandle`, matching the existing `drift_log`/`peer_warnings`/`equivocation_log` pattern. Bad-sig and seq/length rejections route to the existing `PeerWarning::{SignatureInvalid, DecodeFailed}` logs.
- **`crates/kernel` unconditional dep on `crates/distribution`** — the pure tier compiles without iroh; the kernel must hold log state in non-iroh builds too. `network-iroh` keeps `myrhiza-distribution/network-iroh` for the blob tier.
- **Test-utils propagation** — `installed_authors` parameter threaded through `InProcessHarness::spawn_peer` + `IrohHarness::spawn_peer`; `PeerHandle::{revocation_events, publication_events}` accessors.
- **Tests** — MemNetwork-based kernel-tier unit/acceptance tests for apply-on-valid, bad-sig→warning, seq-not-monotonic→drop; plus `crates/kernel/tests/iroh_revocation.rs` over real iroh-gossip (B-10 spec §6.4).

### Explicitly deferred

- **Stale-network mitigation + 24h-stale warning** ([`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.7) — a HeadsSummary-equivalent periodic backfill broadcast on the revocation/publication topics plus timer-based stale detection. Requires a second ticker + new gossip message type; materially expands scope. → **B-12 or later.**
- **Installed-bundle registry + uninstall-prompt rendering / pin flow** ([`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.5 step 7) — the kernel keeps **no** post-install registry today (verified: `install::load` returns a `LoadedBundle` value and nothing retains it). B-11 surfaces `RevocationApplied` carrying `revoked_bundle_hash`; correlating it against installed bundles and rendering the prompt is embedder/UI work. → follow-up polish (the CLI UI surface for criterion 5 already exists per `crates/myrhiza-cli/`).
- **Log persistence across restart** — `RevocationLog`/`PublicationLog` are in-memory and not `Serialize`/`Deserialize`. On restart `last_observed_seq` resets to 0 and prior revocations re-surface. This is **acceptable**: re-applying a revocation is idempotent (re-flagging an already-revoked bundle is harmless), and the blob `MemStore` is ephemeral anyway. Durable logs are B-9 (storage) territory. See §5.
- **Subscription lifecycle on uninstall** — there is no uninstall path in the kernel yet; subscriptions live for the `Runtime`'s lifetime. (Per §10.7 the revocation sub *should* outlive uninstall anyway, to warn before reinstall.)
- **`modules.dep` recursive author subscription** — a module dep's `expected-author` would need its own subscription. Recursive module-dep resolution is itself deferred (the "plan B" marker in `install.rs`); its revocation subscription rides along with it.
- **Forwarding drainer sub-errors as `PeerWarning`** — the per-author drainer task is spawned outside the select loop and has no `&mut self`; transport/decode errors there are `tracing::warn!`-logged and the task continues. Forwarding them through the channel as structured warnings is a follow-up.
- **Cross-checking `revoked_bundle_hash` against the author's `PublicationLog`** — `RevocationLog::apply` does not verify the revoked hash was ever published by the same author. Out of scope (an author may legitimately revoke a hash a peer never saw a publication for, e.g. an out-of-band-shared bundle).

## 3. Scope decisions

These resolve the open design forks surfaced during code mapping. Per CLAUDE.md, each names the runner-up and why it lost.

### 3.1 Transport: new `GossipMessage` variants, not a new `Network` surface

Revocation/publication envelopes ride the **existing** `Network::subscribe`/`publish` path by appending two `GossipMessage` variants (`Revocation`/`Publication`). The `Network` trait, `Subscription` handle, and `IrohNetwork`/`MemNetwork` impls are untouched.

- **Wire-freeze**: new variants append at discriminants 3/4; `Event=0`/`HeadsSummary=1`/`Drift=2` keep their canonical-bincode u32-BE tags. Two new `wire_freeze.rs` tests pin 3/4.
- **Runner-up**: a second `Network` method (or a generic `Subscription<T>` associated type) carrying raw revocation bytes. Rejected — it widens the trait surface and ripples to every caller and both impls, for no benefit over an enum variant. The dep direction `network → distribution` is acceptable because distribution's pure tier has no network dep (no cycle).

### 3.2 Subscription multiplexing: per-author drainer tasks → shared `mpsc` → one select arm

`tokio::select!` needs statically-known futures, but the peer may have N installed authors → 2N subscriptions. Each subscription gets a spawned drainer task (`drain_distribution_sub`) that forwards `(AuthorPubkey, GossipMessage)` into a single shared `mpsc::Receiver`, polled by one new select arm.

- This **mirrors the existing `internal_event_tx` drainer→mpsc→select-arm pattern** already in `runtime.rs` (used for backfill events). One established pattern reused; no new abstraction.
- **Runner-up**: `tokio_stream::StreamMap` / `futures::stream::SelectAll` over the subscriptions-as-streams. Rejected — no precedent anywhere in the codebase; the drainer pattern is already proven here.

### 3.3 Auto-subscribe trigger: `installed_authors` parameter on `Runtime::start`

`Runtime::start` gains `installed_authors: Vec<AuthorPubkey>` (alongside the existing `bootstrap` param). The `Runtime` owns its subscriptions, so it owns the revocation/publication subscriptions too.

- The embedder reads `loaded.manifest.app.author_pubkey` from each installed bundle, decodes it to `AuthorPubkey`, and passes the set to `Runtime::start`. Decoding stays at the call site, consistent with the existing `decode_author_pubkey_hex` path.
- **Runner-up**: a post-construction `RuntimeHandle::subscribe_author_revocation(author)` method, or wiring subscriptions entirely at the embedder layer. Rejected — splitting subscription ownership between `Runtime` and embedder fragments the select loop's lifecycle; the parameter is the same shape as `bootstrap` and keeps all gossip subscriptions inside the `Runtime`.

### 3.4 Edge verification order: `dispatch::verify_*` first, then `apply`

On receive, the handler calls `dispatch::verify_revocation(&ev, &author)` **first** (signature check at the gossip edge) → on `Err`, push `PeerWarning::SignatureInvalid { peer: None }`, drop. On `Ok`, call `RevocationLog::apply` → on `Err` (seq/length, since the signature already verified) push `PeerWarning::DecodeFailed { peer: None }`; on `Ok` insert the new log and surface `RevocationApplied`.

- **Why verify first**: `RevocationLog::apply`'s internal order is *reason-len → seq-monotonic → seq-jump → signature*. A forged-signature event with a stale seq would be classified `SeqNotMonotonic`, not a signature failure — mis-attributing a forgery as a benign duplicate. Verifying at the edge first (the `dispatch` module's stated purpose, B-10 plan T9 / B-4.8 `PeerWarning` discipline) classifies forgeries correctly. `apply` re-verifies redundantly; that redundancy is acceptable (keeps `apply` independently sound for state-tier tests).
- **Runner-up**: call `apply` alone and map `RevocationError::SignatureInvalid → PeerWarning::SignatureInvalid`. Rejected for the mis-classification above.

### 3.5 Outward surface: `Arc<Mutex<Vec<…>>>` poll-logs, not a push channel

`RevocationApplied`/`PublicationAnnounced` are appended to `Arc<Mutex<Vec<…>>>` poll-logs on `Runtime` (writer) and `RuntimeHandle` (reader), exactly like `drift_log`/`peer_warnings`/`equivocation_log`.

- **Why not `watch`**: a `watch` channel keeps only the latest value, silently dropping intermediate events between polls — wrong for discrete event occurrences (a burst of revocations would lose all but the last).
- **Why not `broadcast`**: introduces a push pattern that exists nowhere in this codebase. The spec's phrase "emit to kernel UI surface" maps onto the poll-log — *that is* the kernel UI surface pattern here. Runner-up rejected to avoid a one-off convention.

### 3.6 Type placement: `RevocationApplied`/`PublicationAnnounced` live in `crates/kernel`

They sit in `runtime.rs` alongside `DriftDetected`/`EquivocationFlag`/`PeerWarning`, because `RuntimeHandle` (in the kernel) is their surface. They reference `BlobHash` + `AuthorPubkey` (both in `crates/types`). The kernel already depends on `crates/distribution`, so no inversion.

## 4. Design

### 4.1 Data flow (against real symbols)

```
INSTALL (embedder)
  install::load(&addr) -> LoadedBundle { manifest.app.author_pubkey: String }
  embedder decodes author_pubkey -> AuthorPubkey, collects installed_authors: Vec<AuthorPubkey>

AUTO-SUBSCRIBE (Runtime::start)
  let (distribution_tx, distribution_rx) = mpsc::channel::<(AuthorPubkey, GossipMessage)>(256);
  for A in installed_authors {
      let rsub = erased.subscribe(Topic::from_bytes(derive_revocation_topic(A)), bootstrap.clone()).await?;
      let psub = erased.subscribe(Topic::from_bytes(derive_publication_topic(A)), bootstrap.clone()).await?;
      tokio::spawn(drain_distribution_sub(A, rsub, distribution_tx.clone()));
      tokio::spawn(drain_distribution_sub(A, psub, distribution_tx.clone()));
  }

GOSSIP RECV (remote author A publishes)
  network.publish(Topic::from_bytes(derive_revocation_topic(A)), GossipMessage::Revocation(signed_ev))
  -> drain_distribution_sub on receiving peer: sub.recv() => Ok(Some(GossipMessage::Revocation(ev)))
     -> distribution_tx.send((A, GossipMessage::Revocation(ev)))

SELECT ARM 6 (Runtime::run)
  Some((author, msg)) = self.distribution_rx.recv() => self.handle_distribution_message(author, msg)

DISPATCH
  handle_distribution_message:
    Revocation(ev)  => handle_revocation(author, ev)
    Publication(ev) => handle_publication(author, ev)
    _               => peer_warnings.push(PeerWarning::DecodeFailed { peer: None })

  handle_revocation(author, ev):
    match dispatch::verify_revocation(&ev, &author) {
        Err(_) => peer_warnings.push(PeerWarning::SignatureInvalid { peer: None }),   // drop
        Ok(()) => {
            let prior = revocation_logs.get(&author).cloned().unwrap_or_default();
            match prior.clone().apply(&ev, &author) {
                Ok(new) => {
                    revocation_logs.insert(author, new);
                    revocation_events.lock()?.push(RevocationApplied {
                        author, revoked_bundle_hash: ev.revoked_bundle_hash, revocation_seq: ev.revocation_seq,
                    });
                }
                Err(_) => {  // seq/length; signature already verified
                    peer_warnings.push(PeerWarning::DecodeFailed { peer: None });
                    revocation_logs.insert(author, prior);
                }
            }
        }
    }
  // handle_publication is structurally identical: PublicationLog / PublicationAnnounced.

SURFACE
  host holds RuntimeHandle.revocation_events: Arc<Mutex<Vec<RevocationApplied>>>; polls (or via
  PeerHandle::revocation_events()) to learn which bundle hashes were flagged. Uninstall prompt is downstream.
```

### 4.2 New types (`crates/kernel/src/runtime.rs`)

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevocationApplied {
    pub author: AuthorPubkey,
    pub revoked_bundle_hash: BlobHash,
    pub revocation_seq: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicationAnnounced {
    pub author: AuthorPubkey,
    pub manifest_hash: BlobHash,
    pub version: String,
    pub publication_seq: u64,
}
```

### 4.3 `Runtime` / `RuntimeHandle` field additions

Runtime (writer): `revocation_logs: BTreeMap<AuthorPubkey, RevocationLog>`, `publication_logs: BTreeMap<AuthorPubkey, PublicationLog>`, `distribution_rx: mpsc::Receiver<(AuthorPubkey, GossipMessage)>`, `revocation_events: Arc<Mutex<Vec<RevocationApplied>>>`, `publication_events: Arc<Mutex<Vec<PublicationAnnounced>>>`.

RuntimeHandle (reader): `pub revocation_events: Arc<Mutex<Vec<RevocationApplied>>>`, `pub publication_events: Arc<Mutex<Vec<PublicationAnnounced>>>` (clones of the same `Arc`s).

### 4.4 `drain_distribution_sub`

```rust
async fn drain_distribution_sub<S: Subscription + Send + 'static>(
    author: AuthorPubkey,
    mut sub: S,
    tx: mpsc::Sender<(AuthorPubkey, GossipMessage)>,
) {
    loop {
        match sub.recv().await {
            Ok(Some(msg)) => { if tx.send((author, msg)).await.is_err() { break; } }
            Ok(None) => break,                       // subscription closed
            Err(e) => { tracing::warn!(?e, "distribution sub recv error"); /* continue */ }
        }
    }
}
```

(Exact `Subscription`/`recv` bound matches the trait in `crates/network`; verify the erased `Box<dyn Subscription + Send>` shape at impl time — see the plan's T3.)

## 5. Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `Runtime::start` signature change ripples to ~13 test call sites | High | Mechanical churn | One task (plan T5) does the propagation pass; `vec![]` for all non-revocation callers |
| `GossipMessage` variant insert reorders discriminants | Low | Wire break | Append only (3/4); two `wire_freeze.rs` tests pin them; `Event/HeadsSummary/Drift` tests stay green |
| Log resets on restart re-surface old revocations | Medium | Duplicate uninstall prompts | Acceptable: re-flagging is idempotent; documented as deferred (B-9 persistence). `MemStore` is ephemeral too |
| Double signature verify (`verify_*` then `apply`) | Low | Minor CPU | Accepted — keeps `apply` independently sound; revocation volume is tiny |
| iroh-gossip swarm not formed before publish in test | Medium | Flaky test | Publisher subscribes to the same topic with peer-A bootstrap; settle sleep + bounded poll loop (mirrors `iroh_convergence.rs`) |
| `RevocationLog::apply` consumes `self` | — | API friction | Clone prior log before `apply`; re-insert prior on `Err` (the only correct pattern for the consume-and-return API) |

## 6. Test plan

### 6.1 Wire-freeze (`crates/types/tests/wire_freeze.rs`)
- `gossip_message_revocation_variant_tag_is_three_u32_be` — `canonical_bincode(GossipMessage::Revocation(zero_sig_ev))[..4] == [0,0,0,3]`.
- `gossip_message_publication_variant_tag_is_four_u32_be` — `[0,0,0,4]`.

### 6.2 Kernel-tier, MemNetwork (fast, no iroh)
- `revocation_applied_on_valid_event` — `InProcessHarness`, peer with `installed_authors=[A]`; publish `GossipMessage::Revocation(valid_signed_ev)`; assert `peer.revocation_events()` gains `RevocationApplied`.
- `invalid_sig_revocation_becomes_peer_warning` — wrong signing key; assert `revocation_events` empty + `peer_warnings` gains `SignatureInvalid`.
- `seq_not_monotonic_second_event_dropped` — two events seq=1; assert exactly one `RevocationApplied`.
- Publication analogues for the first two.

### 6.3 Kernel-tier, real iroh-gossip (`crates/kernel/tests/iroh_revocation.rs`, `#[cfg(feature = "network-iroh")]`)
Per B-10 spec §6.4. Four tests: `revocation_gossip_applies_and_surfaces`, `publication_gossip_applies_and_surfaces`, `invalid_signature_becomes_peer_warning`, `seq_not_monotonic_second_event_dropped`. Sign with `deterministic_signing_key(7)` (matches `build_signed_counter_bundle` author). Receiving peer via `IrohHarness::spawn_peer(installed_authors=[A])`; publishing peer via `spawn_iroh_peer` directly to reach `IrohPeerStack::network` for the raw `publish`. Publisher subscribes the same topic (peer-A bootstrap) so Plumtree routing delivers. Bounded poll loop with timeout.

### 6.4 What is NOT tested
- Cross-process (E2E-2 territory). Restart persistence (B-9). Stale-network backfill (deferred §2). Flood beyond the unit seq-jump check.

## 7. Surface change summary

### New public surface
- `myrhiza_kernel::runtime::{RevocationApplied, PublicationAnnounced}`.
- `RuntimeHandle::{revocation_events, publication_events}` fields.
- `PeerHandle::{revocation_events(), publication_events()}` (test-utils).
- `myrhiza_network::GossipMessage::{Revocation, Publication}` variants.

### Modified public surface
- `Runtime::start` gains `installed_authors: Vec<AuthorPubkey>`.
- `InProcessHarness::spawn_peer` + `IrohHarness::spawn_peer` gain `installed_authors: Vec<AuthorPubkey>`.
- `crates/kernel` + `crates/network`: `myrhiza-distribution` becomes an unconditional dep.

### Unchanged
- `Network` trait + `Subscription` handle + both impls.
- `install::load` signature. `RevocationLog`/`PublicationLog`/`dispatch::*` (consumed as-is).
- All existing wire-freeze discriminants 0/1/2.

## 8. Cross-references

- [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.5 (install flow step 7 — UI surface), §10.7 (revocation distribution + threat model + seq/jump caps + stale-network mitigation).
- [B-10 spec](2026-05-26-b-10-bundle-distribution-design.md) §2 (the unshipped in-scope items), §4.4 (revocation schema + state machine), §4.5 (publishing sequence), §6.4 (revocation propagation test), §10 ("Revocation surfaces to UI" deferral), §12.
- [B-4.1 spec](2026-05-20-plan-b-4-1-iroh-gossip-design.md) — the `IrohNetwork::publish`/`subscribe` surface reused. [B-4.4 spec](2026-05-21-plan-b-4-4-direct-streams-design.md) — the drainer→mpsc→select-arm pattern §3.2 mirrors.
- [B-4.2 attribution](2026-05-20-plan-b-4-2-attribution-design.md) / B-4.8 — `PeerWarning` discipline reused for bad-sig/decode rejections.
- [E2E coverage spec](2026-05-22-e2e-test-coverage-design.md) — the `IrohHarness` pattern §6.3 extends.
- [MVP gap analysis](../reports/2026-05-21-mvp-gap-analysis.md) — item 14 footnote naming this follow-up.

## 9. Prior-art

This is a wiring slice over already-decided design; the load-bearing prior-art consultation lives in [B-10 spec §9](2026-05-26-b-10-bundle-distribution-design.md). Re-confirmed relevant: [`prior-art/app-distribution/open-problems.md`](../prior-art/app-distribution/open-problems.md) §3 ("Component-bundle revocation" — *"In-band negative events… an event in the app's own log says 'revoked'"*); [`prior-art/iroh/`](../prior-art/iroh/) gossip routing (Plumtree delivery the test relies on). No new external systems in scope.

## 10. Out-of-scope future work — explicit deferrals

Mirrors §2 "Explicitly deferred" — promoted here for the catalog. Each is a candidate child slice:

- **B-12 (proposed): revocation stale-network backfill** — HeadsSummary-shape periodic re-broadcast on revocation/publication topics + 24h-stale warning before install ([`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.7 stale-network attack).
- **Installed-bundle registry + uninstall/pin UI** — kernel-side post-install registry, then the §10.5 step 7 prompt + pin opt-in.
- **Durable `RevocationLog`/`PublicationLog`** — `Serialize`/`Deserialize` + storage-backed load on restart (folds into B-9 `crates/storage`).
- **Drainer sub-error → structured `PeerWarning`** forwarding.
- **Module-dep author revocation subscription** (rides with recursive module-dep resolution).
- **Cross-check revoked hash against `PublicationLog`** (if ever desired).

## 11. Estimate

**2–3 days** for one focused engineer (gap-analysis candidate-#1 estimate ~3–5 days; the precise blueprint trims it). Breakdown matches the plan's T1–T7:

- T1: `GossipMessage` variants + network dep + wire-freeze tests — ~0.5 day.
- T2–T4: kernel dep + types + `Runtime` fields + `start` subscription wiring + select arm + handlers — ~1 day.
- T5: call-site propagation + `PeerHandle` accessors + green full suite — ~0.5 day.
- T6: `iroh_revocation.rs` (the load-bearing test, swarm-timing shake-out) — ~0.5–1 day.
- T7: fmt + clippy zero-warnings + both feature matrices green + docs — ~0.25 day.

## 12. Open questions for the plan writer

Decisions the spec intentionally leaves to execution:

1. **Erased `Subscription` bound** — confirm the exact `Box<dyn Subscription + Send>` / `recv()` shape the existing `erased.subscribe(...)` returns, and whether `drain_distribution_sub` takes the erased box or a generic `S: Subscription`. (T3.)
2. **Settle timing in `iroh_revocation.rs`** — sleep + poll-loop bounds; tune against observed iroh-gossip swarm-formation latency (mirror `iroh_convergence.rs`'s 300ms). Capture observed numbers in the PR body.
3. **`MemNetwork` topic routing for distribution topics** — verify `MemNetwork::publish`/`subscribe` route by exact topic bytes so the per-author topic is isolated from the app topic in the in-process harness. (T4 test depends on it.)
4. **Whether `handle_revocation`/`handle_publication` share a generic helper** — they are structurally identical modulo types; the plan-writer may factor a generic or keep two explicit methods (lean explicit for clarity).

## Sources

- B-10 design + plan (predecessor); `crates/distribution/src/{revocation,publication,dispatch,topic,signed_envelope}.rs` (consumed API, verified 2026-05-28).
- `crates/kernel/src/runtime.rs` (select loop + `RuntimeHandle` poll-log pattern), `crates/network/src/lib.rs` (`GossipMessage` + `Network` trait), `crates/test-utils/src/{iroh_harness,harness,bundle}.rs` (test harness).
- [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.5 + §10.7 (normative revocation flow).
