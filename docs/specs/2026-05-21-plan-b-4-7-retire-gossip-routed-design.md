**Date:** 2026-05-21
**Status:** draft
**Parent:** [docs/specs/2026-05-09-myrhiza-master-design/README.md](2026-05-09-myrhiza-master-design/README.md)
**Subject:** Plan B-4.7 — Retire the gossip-routed HeadsRequest surface

# Plan B-4.7 design — Retire the gossip-routed HeadsRequest surface

## 1. Goal

Final B-4 slice. Remove the entire gossip-routed `HeadsRequest` codepath now that direct-streams own backfill end-to-end:

1. **Wire**: remove `GossipMessage::HeadsRequest`, `HeadsRequest`, and `HeadsRequestSignedPayload`. Regenerate `wire_freeze.rs` snapshots (the `GossipMessage` enum's remaining variants get new tags: 0 Event, 1 HeadsSummary, **2 Drift** (was 3); HeadsRequest's slot at 2 is gone).
2. **Runtime**: remove `handle_heads_request`, `build_signed_heads_request`, `verify_heads_request`, and the `GossipMessage::HeadsRequest(_)` arm in `handle_message`.
3. **Fallback semantic**: when `request_author_chain_gap` fires with an empty `peer_authority_index` (no known peer for the author), replace the previous gossip-routed `publish(GossipMessage::HeadsRequest)` with a soft-nudge `publish_heads_summary()` — the same recovery primitive the cross-author Pending path already uses. Peers receiving our HeadsSummary will diff their tips and (a) populate THEIR peer-authority index with our pubkey, then (b) issue a direct-stream backfill to us if they're behind, OR push HeadsSummaries back at us that will populate OUR index for future recovery.
4. **Tests**: update `peer_authority_index.rs::pending_event_with_unknown_author_falls_back_to_gossip` to assert the new soft-nudge behavior (HeadsSummary published, not HeadsRequest). Update `convergence.rs::pending_event_triggers_heads_request_not_heads_summary` — the name + assertion no longer match reality; rename or rework.
5. **Cleanup**: remove the now-dead spec comments + plan references about "fallback gossip path" in `runtime.rs`'s loopback-guard comment, etc.

This is the project's first deliberate wire-format change since the genesis of the wire-freeze contract. Per the B-4.2 §10 "Wire-version envelope — first post-1.0 wire change becomes the trigger" deferral, we DO NOT introduce a version envelope in this slice — Myrhiza is pre-launch; the wire-freeze snapshot serves as a regression gate, not a versioning surface. B-4.7 regenerates the snapshot; downstream binaries that share this main branch must rebuild against it.

## 2. Scope decisions (locked during brainstorming + B-4.6 runtime survey, 2026-05-21)

| Decision | Chosen | Runner-up | Why |
|---|---|---|---|
| **Remove HeadsRequest cleanly (no tombstone variant)** | Delete `GossipMessage::HeadsRequest`, the wire types, and all helpers. Regenerate `wire_freeze.rs`. | (a) Leave a `#[doc(hidden)] HeadsRequest_Removed_DoNotUse` tombstone with empty body; (b) Mark the variant `#[deprecated]` and keep code. | Tombstoning preserves variant tag stability but creates a permanent dead-code attractor. Pre-launch, we have no inbound traffic from external peers; the only consumers are this codebase. Clean removal is the right shape; the wire-freeze regen makes the cut explicit. The B-4.2 spec's "wire-version envelope" deferral explicitly anticipates this: "First post-launch wire change becomes the trigger to land [the version envelope]." We are pre-launch; B-4.7 is the first such change, but the envelope is not the trigger — the next post-launch wire change is. |
| **Empty-index fallback = `publish_heads_summary()` soft nudge** | Replace the gossip-routed publish with `let _ = self.publish_heads_summary().await;`. Matches the existing cross-author Pending recovery at `request_missing_for` line 1101. | (a) No-op + log a `PeerWarning::PeerAuthorityIndexEmpty`; (b) Periodic retry; (c) Trigger discovery (B-4.8+). | The cross-author Pending and the empty-index same-author cases converge to "we need help from somebody; broadcast our state and let peers diff against it." HeadsSummary is exactly that primitive. No new warning variant needed (failure mode is implicit in the soft-nudge timing). Periodic retry happens naturally — every `heads_summary_tick` (default 1s) we publish our HeadsSummary. Discovery is a future slice. |
| **Update `request_missing_for` for symmetry** | After B-4.7, the same-author gap branch may now also fall through to publish_heads_summary (via request_author_chain_gap's new empty-index handler). The cross-author branch already does. Both branches converge to the same recovery; consider simplifying. | Leave both branches as-is. | Cleaner code path is appealing but the current shape has documented review-finding I-1 origin (plan-B-1-fixes Task 4). Don't refactor. Just leave both branches calling the (now-simpler) helpers; the duplication is intentional for clarity. |
| **Remove `verify_heads_request`** | Yes — no caller after `handle_heads_request` is gone. | Keep as defense-in-depth. | Dead code attracts confusion. The `HeadsRequestSignedPayload` type goes with it (private signing-payload type, no other consumers). |
| **Wire-freeze snapshot is fully regenerated** | Run `cargo test wire_freeze` after deletion; capture new hex; paste into the test file; re-run. Snapshot is per-existing-pattern. | Manually compute. | Regeneration is the established workflow; trying to hand-compute the new bytes would introduce errors. |
| **`net_pub` / `net_tap` test patterns unaffected** | These tests use the broadcast publish + tap pattern with `GossipMessage::Event` / `GossipMessage::HeadsSummary`. Neither relied on `HeadsRequest`. | Per-test audit. | Quick grep confirms no test outside the two flagged tests references `GossipMessage::HeadsRequest`. The two tests (one in `peer_authority_index.rs`, one in `convergence.rs`) are updated explicitly. |
| **Wire-version envelope NOT added in B-4.7** | Defer per B-4.2 §10. | Add now while we're touching wire format. | The envelope is a forward-compat tool for POST-launch wire changes (rolling deployment across peers running different versions). Pre-launch, the wire-freeze snapshot serves as the gate. Adding the envelope would itself be a wire change without a downstream rollout to validate against. |
| **No PeerWarning::PeerAuthorityIndexEmpty for the empty-index case** | Defer (per B-4.6's same decision). | Add. | The empty-index case is now a normal recovery path (publish HeadsSummary); no failure to surface. Eviction-on-failure (B-4.6 §10 deferral) is the more useful future warning. |
| **No backwards compatibility for pre-B-4.7 peers** | Decoding `GossipMessage` from a pre-B-4.7 emitter will produce `SubError::DecodeFailed` (unknown variant tag). Acceptable pre-launch. | Tombstone variant + warning. | The peer-authority index population happens BEFORE the variant arm in `handle_message`. A pre-B-4.7 peer's HeadsSummary still parses correctly (no enum change there). Only their HeadsRequests fail to decode — and we no longer service them anyway. So the failure mode is symmetric. |
| **Wire-freeze docstring update** | The comments in `wire_freeze.rs` that list variant tags (`GossipMessage::HeadsRequest = 2`, `GossipMessage::Drift = 3`) get updated to `Drift = 2` after the deletion. | Leave docstring stale. | Stale doc misleads. Keep the truth. |

## 3. Code surface

### 3.1 `request_author_chain_gap` — `crates/kernel/src/runtime.rs`

Current (post-B-4.6, around line 1130):

```rust
async fn request_author_chain_gap(&mut self, author: AuthorPubkey, from_seq: u64, to_seq: u64) {
    if to_seq < from_seq || to_seq == 0 {
        return;
    }
    let mut requests = Vec::new();
    Self::paginate_into(author, from_seq, to_seq, &mut requests);
    if requests.is_empty() {
        return;
    }

    if let Some(target_peer) = self.lookup_peer_for_author(&author) {
        self.issue_direct_backfill(target_peer, requests).await;
        return;
    }

    let Ok(req) = self.build_signed_heads_request(requests) else {
        return;
    };
    let _ = self
        .network
        .publish(self.topic, GossipMessage::HeadsRequest(req))
        .await;
}
```

After B-4.7:

```rust
async fn request_author_chain_gap(&mut self, author: AuthorPubkey, from_seq: u64, to_seq: u64) {
    if to_seq < from_seq || to_seq == 0 {
        return;
    }
    let mut requests = Vec::new();
    Self::paginate_into(author, from_seq, to_seq, &mut requests);
    if requests.is_empty() {
        return;
    }

    if let Some(target_peer) = self.lookup_peer_for_author(&author) {
        self.issue_direct_backfill(target_peer, requests).await;
        return;
    }

    // Empty index — no peer known to have authority over this author.
    // Soft-nudge: publish our HeadsSummary so peers can diff and either
    // push their HeadsSummaries back (which will populate our index for
    // a future recovery attempt) or, if they're also behind on this
    // author, propagate the gap further. Matches the cross-author Pending
    // recovery in `request_missing_for` (runtime.rs:1101). Per B-4.7 spec §3.1.
    let _ = self.publish_heads_summary().await;
}
```

The `requests: Vec<EventRequest>` becomes unused on the fallback path; either drop it (after `paginate_into`) before the soft-nudge or keep it for future restructuring. **Choose: drop the unused builder and just call `publish_heads_summary()`.** The `paginate_into` call is still useful as a sanity check that the requested range is non-empty.

Actually simpler: drop the entire `requests` building when we know we'll soft-nudge. Restructure:

```rust
async fn request_author_chain_gap(&mut self, author: AuthorPubkey, from_seq: u64, to_seq: u64) {
    if to_seq < from_seq || to_seq == 0 {
        return;
    }
    let Some(target_peer) = self.lookup_peer_for_author(&author) else {
        // Empty index — soft-nudge via HeadsSummary.
        // Per B-4.7 spec §3.1.
        let _ = self.publish_heads_summary().await;
        return;
    };
    let mut requests = Vec::new();
    Self::paginate_into(author, from_seq, to_seq, &mut requests);
    if requests.is_empty() {
        return;
    }
    self.issue_direct_backfill(target_peer, requests).await;
}
```

Moving the lookup BEFORE the paginate avoids building requests we won't use. Cleaner.

### 3.2 Remove `build_signed_heads_request` — `crates/kernel/src/runtime.rs`

Find the method (around `runtime.rs:744`) and delete it. Imports of `HeadsRequest` + `HeadsRequestSignedPayload` from `myrhiza_types` get pruned in §3.4.

### 3.3 Remove `handle_heads_request` and the dispatch arm — `crates/kernel/src/runtime.rs`

Find `handle_message` (currently dispatches 4 variants). Remove the `GossipMessage::HeadsRequest(r) => { ... }` arm:

```rust
async fn handle_message(&mut self, msg: GossipMessage) -> Result<(), RuntimeError> {
    match msg {
        GossipMessage::Event(e) => self.handle_event(e).await?,
        GossipMessage::HeadsSummary(h) => {
            if self.verify_heads_summary(&h) {
                self.handle_heads_summary(h).await?;
            }
        }
        // GossipMessage::HeadsRequest arm removed (B-4.7).
        GossipMessage::Drift(d) => self.process_drift_message(d).await,
    }
    Ok(())
}
```

Then delete `async fn handle_heads_request(...)` (around `runtime.rs:1222`).

Also delete `fn verify_heads_request(...)` (around `runtime.rs:1615`) — no remaining caller.

### 3.4 Remove wire types — `crates/types/src/dag.rs`

Delete:
- `pub struct HeadsRequest { ... }`
- `pub struct HeadsRequestSignedPayload { ... }`
- The corresponding `tests_drift_heads` test cases (`heads_request_round_trips`, `heads_request_signed_payload_round_trips`).

Update `crates/types/src/lib.rs` re-exports — drop `HeadsRequest` and `HeadsRequestSignedPayload`.

### 3.5 Remove `GossipMessage::HeadsRequest` variant — `crates/network/src/lib.rs`

```rust
pub enum GossipMessage {
    Event(Event),
    HeadsSummary(HeadsSummary),
    // HeadsRequest variant removed (B-4.7).
    Drift(DriftMessage),
}
```

Drop the `HeadsRequest` import from `myrhiza_types::{...}` at the top of the file.

### 3.6 Wire-freeze regen — `crates/types/tests/wire_freeze.rs`

After deletions:

1. Remove the `heads_request_wire_freeze` and `heads_request_signed_payload_wire_freeze` test cases.
2. Remove the helper `fn sample_heads_request()`.
3. Update the `GossipMessage` variant-tag tests:
   - Remove the `gossip_message_heads_request_variant_tag_is_2` test.
   - Update the comment block listing variant tags (Event=0, HeadsSummary=1, Drift=2 after removal).
   - The existing `gossip_message_drift_variant_tag_is_3` test must update to `gossip_message_drift_variant_tag_is_2` and regenerate its hex.
4. Run `cargo test -p myrhiza-types --test wire_freeze` once to fail with the new hex output captured; paste in; re-run to PASS.

### 3.7 Update existing tests that rely on the gossip-routed path

- **`crates/kernel/tests/convergence.rs::pending_event_triggers_heads_request_not_heads_summary`** (around line 541):
  - The assertion that the tap captures `GossipMessage::HeadsRequest` is INVALID after B-4.7 — that variant no longer exists.
  - Two options:
    - (a) Update to assert `GossipMessage::HeadsSummary` (the new soft-nudge).
    - (b) Delete the test entirely; the B-4.6 peer_authority_index.rs tests cover the substantive recovery paths.
  - **Pick (a)**: keep the test as a regression guard on the soft-nudge behavior. Rename to `pending_event_triggers_heads_summary_nudge_when_index_empty`. Update the docstring + assertion. The test's existing setup leaves the index empty (per audit), so this is a natural rename.

- **`crates/kernel/tests/peer_authority_index.rs::pending_event_with_unknown_author_falls_back_to_gossip`** (test 5):
  - Currently asserts `GossipMessage::HeadsRequest` arrives on the tap.
  - Update to assert `GossipMessage::HeadsSummary` (the new soft-nudge). Rename to `pending_event_with_unknown_author_publishes_heads_summary_nudge`.

### 3.8 Cleanup of stale B-4.6 comments — `crates/kernel/src/runtime.rs`

The fallback-gossip comment in `request_author_chain_gap` is replaced (covered in §3.1). Other comments that mention "the gossip-routed fallback" or "B-4.7 retires" should be removed or updated to past tense ("B-4.7 retired ...").

## 4. Acceptance tests

### 4.1 No new test file

B-4.7 is a removal slice. The existing tests are updated per §3.7. No new tests added.

### 4.2 Updated tests
- `convergence.rs::pending_event_triggers_heads_summary_nudge_when_index_empty` (renamed from `_heads_request_not_heads_summary`)
- `peer_authority_index.rs::pending_event_with_unknown_author_publishes_heads_summary_nudge` (renamed from `_falls_back_to_gossip`)

### 4.3 Wire-freeze regen verification

The `wire_freeze` tests must all PASS after regeneration. Run:

```bash
cargo test -p myrhiza-types --test wire_freeze
```

Expect: all PASS with the new hex.

### 4.4 Full workspace test

`cargo test --workspace --all-features` MUST be clean — every direct or indirect consumer of `HeadsRequest` / `GossipMessage::HeadsRequest` has been updated.

## 5. Justfile changes

`just spec-coverage` regeneration after B-4.7 lands.

## 6. Edge cases

| Scenario | Behavior |
|---|---|
| Pre-B-4.7 peer sends `GossipMessage::HeadsRequest` | `IrohSubscription::recv` or `MemSubscription::recv` returns `Err(SubError::DecodeFailed { peer })` (unknown variant tag). Runtime logs `PeerWarning::DecodeFailed`. Acceptable pre-launch failure mode; no peers to worry about. |
| Empty index + same-author gap | `publish_heads_summary` fires. Peer that has authority over the author publishes HeadsSummary back; we populate our index. On the NEXT same-author gap (or after `heads_summary_tick` retries), the direct-stream path succeeds. |
| Empty index + cross-author Pending | Unchanged behavior — `request_missing_for`'s `else` branch already does `publish_heads_summary`. |
| All peers behind the same author | `publish_heads_summary` from all of them spreads the news that "we're all behind." Eventually whoever is ahead (or the author themselves on reconnect) publishes their HeadsSummary; the chain converges. |
| Wire-freeze regen fails | Inspect the diff between old and new hex; verify the only change is removed variants + remaining-variant tag shift. Commit the new hex with a clear "BEHAVIOR CHANGE" annotation in the wire-freeze comment block. |

## 7. Surface change summary

**Removed crate-public surface**:

- `myrhiza_types::HeadsRequest` (struct).
- `myrhiza_types::HeadsRequestSignedPayload` (struct).
- `myrhiza_network::GossipMessage::HeadsRequest` (variant).
- `myrhiza_kernel::runtime::Runtime::build_signed_heads_request` (private; was already pub(crate)-equivalent).
- `myrhiza_kernel::runtime::Runtime::verify_heads_request` (private).
- `myrhiza_kernel::runtime::Runtime::handle_heads_request` (private).

**Wire-format change**:

- `GossipMessage` enum: variant tag 2 (HeadsRequest) removed. Variant tag 3 (Drift) shifts to 2.

**Behavior change**:

- `request_author_chain_gap` with empty `peer_authority_index` now publishes a HeadsSummary nudge instead of a HeadsRequest broadcast.

**Unchanged**:

- `Network` trait surface.
- `RuntimeHandle` shape.
- `peer_authority_index` semantics.
- All direct-stream surface from B-4.4/4.5/4.6.

## 8. Non-goals (explicit)

- **Wire-version envelope** — defer per B-4.2 §10. The first POST-launch wire change is the trigger.
- **Per-requester rate limiting** at the responder — defer.
- **Eviction-on-DirectRequestFailed for peer-authority index** — defer.
- **PeerWarning::PeerAuthorityIndexEmpty** — defer.
- **Cross-process / cross-machine tests** — defer.
- **Discovery / pkarr / DHT integration** — defer.
- **`Lagged`-event mapping test on real iroh-gossip** (B-4.1 §4 carryover) — defer.
- **`PeerWarning::SignatureInvalid` backfill in `process_drift_message`** (B-4.2 §10 carryover) — defer.
- **`HeadsStreamError::Handler` variant population** (B-4.4 final-review reservation) — defer.

## 9. Prior-art consultation

- [`prior-art/iroh/gossip.md`](../prior-art/iroh/gossip.md) §"Topic IDs are a flat namespace with no auth" — confirms the protocol layer doesn't carry our authority concept.
- [`docs/specs/2026-05-20-plan-b-4-2-attribution-design.md`](2026-05-20-plan-b-4-2-attribution-design.md) §10 — wire-version-envelope deferral statement.
- [`docs/specs/2026-05-21-plan-b-4-6-peer-authority-index-design.md`](2026-05-21-plan-b-4-6-peer-authority-index-design.md) §10 — B-4.7 scope sketch.

## 10. Future work — explicit deferrals

- **Wire-version envelope** — first POST-launch wire change.
- **Per-requester rate limiting**.
- **Eviction-on-DirectRequestFailed** for peer-authority index.
- **Discovery / pkarr / DHT integration**.
- **`Lagged`-event mapping test on real iroh-gossip** (B-4.1 §4 carryover).
- **`PeerWarning::SignatureInvalid` backfill in `process_drift_message`** (B-4.2 §10 carryover).
- **`HeadsStreamError::Handler` variant population** (B-4.4 final-review reservation).
- **MVP-completion review** — after B-4.7 lands, B-4 sequence is done. Survey what's left for the launch readiness check.

## 11. Sources

- `crates/kernel/src/runtime.rs:744` — `build_signed_heads_request` (deleted in §3.2).
- `crates/kernel/src/runtime.rs:870` — `handle_message` dispatch (modified in §3.3).
- `crates/kernel/src/runtime.rs:1130` — `request_author_chain_gap` (modified in §3.1).
- `crates/kernel/src/runtime.rs:1222` — `handle_heads_request` (deleted in §3.3).
- `crates/kernel/src/runtime.rs:1615` — `verify_heads_request` (deleted in §3.3).
- `crates/types/src/dag.rs` — `HeadsRequest` + `HeadsRequestSignedPayload` (deleted in §3.4).
- `crates/network/src/lib.rs` — `GossipMessage::HeadsRequest` (deleted in §3.5).
- `crates/types/tests/wire_freeze.rs` — regenerated in §3.6.
- `crates/kernel/tests/convergence.rs:541` — renamed/updated in §3.7.
- `crates/kernel/tests/peer_authority_index.rs` (test 5) — renamed/updated in §3.7.
- [`docs/specs/2026-05-20-plan-b-4-2-attribution-design.md`](2026-05-20-plan-b-4-2-attribution-design.md) §10.
- [`docs/specs/2026-05-21-plan-b-4-6-peer-authority-index-design.md`](2026-05-21-plan-b-4-6-peer-authority-index-design.md) §10.
