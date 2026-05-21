**Date:** 2026-05-21
**Status:** draft
**Parent:** [docs/specs/2026-05-09-myrhiza-master-design/README.md](2026-05-09-myrhiza-master-design/README.md)
**Subject:** Plan B-4.6 — Peer-authority index + convert `request_author_chain_gap` to direct-streams

# Plan B-4.6 design — Peer-authority index for Pending/InvalidChain backfill

## 1. Goal

Convert the second (and last) gossip-routed backfill emission path — `request_author_chain_gap` — to direct-streams. This is the path that fires when `handle_event` sees a Pending or `InvalidChain` outcome and recovers by asking some peer for the missing chain range.

B-4.5 converted the HeadsSummary-driven path; that path had `signed_by_peer` as a natural target. The Pending/InvalidChain path doesn't — when an Event arrives, the runtime only sees `event.author` (the original author of the event), not which peer last forwarded it. We need a new source of target peers.

B-4.6 introduces a **peer-authority index**: a `BTreeMap<AuthorPubkey, Vec<PeerPubkey>>` on the runtime, populated incrementally as we observe signed HeadsSummaries. Each HeadsSummary advertises `signed_by_peer` and a list of `AuthorHead`s — that's an attested mapping "peer X claims to have events for authors A, B, C." We persist that mapping in the index and use it as the target source for `request_author_chain_gap`.

**Out of scope (deferred to B-4.7)**:
- Removal of the gossip-routed inbound `handle_heads_request`. Stays active for backwards compatibility + servicing requests from peers running pre-B-4.6 code.
- Removal of the `GossipMessage::HeadsRequest` wire variant. Requires a wire-freeze regeneration.
- Removal of `build_signed_heads_request`. After B-4.6, the only caller is `request_author_chain_gap`'s fallback path; once that fallback is removed (B-4.7), the helper goes too.
- Per-requester rate limiting at the responder side.
- Discovery / pkarr / DHT integration.
- Cross-process / cross-machine tests.

## 2. Scope decisions (locked during brainstorming + B-4.5 runtime survey, 2026-05-21)

| Decision | Chosen | Runner-up | Why |
|---|---|---|---|
| **Peer-authority index, not last-hop attribution** | Track `BTreeMap<AuthorPubkey, Vec<PeerPubkey>>` on Runtime, populated from `HeadsSummary` receipts. | Extend `Subscription::recv` to return `(GossipMessage, Option<PeerPubkey>)` (the iroh-gossip `delivered_from` last-hop neighbor) and use that as the target. | Last-hop attribution is a SemVer break to the `Subscription` trait + cascades through every consumer call site. Plumtree's last-hop is a forwarder, not guaranteed to have the specific events being requested (especially if the event itself reached us via several hops). The peer-authority index is more accurate: a peer that signed a HeadsSummary advertising authority over an author IS attesting "I have events for this author" — better signal than last-hop. The new state is small (BTreeMap with bounded Vec per entry). |
| **Most-recently-observed first, capped at 8 peers per author** | `Vec<PeerPubkey>` with the most-recent-observation at index 0. On overflow, drop index 7 (least-recently-observed). On re-observation (same peer for same author), move to front. | (a) `BTreeSet<PeerPubkey>` (no ordering, no cap); (b) `Vec` with random selection; (c) LRU eviction with `lru` crate. | Most-recent ordering reflects "this peer is alive RIGHT NOW" — the strongest liveness signal we have. Cap at 8 bounds memory per author (8 × 32 bytes = 256 bytes per known author); for an app with thousands of authors, total index size stays under 256 KiB. BTreeSet would lose the liveness ordering. Random selection over the full set would pick stale peers. The `lru` crate adds a workspace dep for a bounded-Vec we can implement in 30 lines. |
| **Index populated in `handle_heads_summary` BEFORE the diff is computed** | Right at the top of `handle_heads_summary` (after `check_fuel_table_version`, before building the local map): for each `AuthorHead` in `remote.authors`, call `self.record_peer_authority(remote.signed_by_peer, head.author)`. | Populate after the diff (so we only learn about peers whose summaries we ACTED on). | Recording authority is independent of whether we needed backfill from this summary — even if we're up-to-date on every author the remote advertised, that peer demonstrably HAS those events and may be useful for a future Pending. Recording early is strictly more information. The order matters only for the loopback case (we never record OURSELVES because the verify-side filter at line 1836 rejects self-signed before `handle_heads_summary` runs). |
| **Pick the head of the Vec; fallback to gossip-routed `publish(GossipMessage::HeadsRequest)` if Vec is empty** | `request_author_chain_gap` calls `lookup_peer_for_author(author) -> Option<PeerPubkey>` (returns the head of the Vec, if any). If `Some(peer)`: `issue_direct_backfill(peer, requests)`. If `None`: legacy `publish(GossipMessage::HeadsRequest)` for backwards compatibility. | (a) No fallback — just log + drop the recovery attempt; (b) Always do both (direct-stream + broadcast); (c) Publish HeadsSummary as a soft nudge. | The fallback to gossip-routed preserves the existing recovery semantic for the case where the runtime has never observed a HeadsSummary covering this author. Without the fallback, a peer that joins mid-stream and observes an Event before any HeadsSummary would have no recovery path. Always-both doubles network traffic; HeadsSummary-as-soft-nudge is the existing cross-author Pending behavior (different path). The fallback path is what B-4.7 will remove once the index is proven; it's our safety net for B-4.6. |
| **Re-observation MOVES the peer to front; does not duplicate** | `record_peer_authority(peer, author)`: if `peer` already in the Vec, remove it then push to front; else push to front; if Vec.len() > 8, pop the tail. | (a) Always push (creating duplicates); (b) Use BTreeSet then push to a separate "recency Vec". | The Vec-with-MRU pattern is the classic "small bounded LRU cache" idiom — O(n) for the move, but n ≤ 8 so it's effectively constant. Duplicates would distort the per-peer reliability signal and waste cap slots. |
| **No DirectRequestFailed → eviction in B-4.6** | When `issue_direct_backfill` fails (returns Err and pushes DirectRequestFailed warning), the peer STAYS in the index at its current position. | Move the failed peer to the tail; or evict immediately. | Eviction-on-failure interacts with transient outages: a temporary network blip could evict every peer for an author, making future recovery impossible. The current B-4.5 retry semantic (next HeadsSummary tick triggers another attempt; if THAT also returns DirectRequestFailed, the warning logs again) is sufficient for B-4.6. Real eviction policy is post-launch tuning. |
| **`Runtime::peer_authority_index: BTreeMap<AuthorPubkey, Vec<PeerPubkey>>` field** | New field on Runtime. Initialized empty in `Runtime::start`. | `Arc<Mutex<BTreeMap<...>>>` for shared access. | Only the runtime task reads/writes the index. `&mut self` access in the select loop is sufficient. Mutex would add lock overhead for no gain. |
| **`record_peer_authority` is `&mut self`, called only from `handle_heads_summary`** | One call site, runtime-task-local. | Expose for tests via `pub(crate)`. | Tests can drive HeadsSummary through the bus naturally; no need for direct access. Keeps the public surface clean. |
| **`lookup_peer_for_author` is `&self`, returns `Option<PeerPubkey>`** | Returns the head of the Vec for the author (most-recently-observed). | Return a `Vec<PeerPubkey>` and let the caller choose. | The caller (`request_author_chain_gap`) always picks one. Returning multiple shifts the policy decision to the caller without any current consumer needing the choice. |
| **Tests use MemNetwork end-to-end; no new harness** | Same pattern as B-4.5's `direct_backfill.rs`: MemBus + multiple MemNetworks + Runtimes; trigger Pending/InvalidChain via crafted Event injection. | Iroh-tier integration test. | The protocol shape is what's being verified; MemNetwork exercises the new index path + direct-stream call + drainer fully. Iroh integration is implicitly exercised by the B-4.4 + B-4.5 acceptance tests already on real iroh. |

## 3. Code surface

### 3.1 New field on `Runtime` — `crates/kernel/src/runtime.rs`

```rust
struct Runtime {
    // ... existing fields ...

    /// Peer-authority index: for each author, the list of peers
    /// observed to have signed a `HeadsSummary` advertising authority
    /// over that author. Ordered most-recently-observed first; capped
    /// at 8 entries per author (least-recently-observed evicted on
    /// overflow). Populated by [`Self::record_peer_authority`] from
    /// `handle_heads_summary`. Queried by
    /// [`Self::lookup_peer_for_author`] from
    /// `request_author_chain_gap` to pick a direct-stream target.
    /// Per B-4.6 spec §3.1.
    peer_authority_index: BTreeMap<AuthorPubkey, Vec<PeerPubkey>>,
}
```

Initialize in `Runtime::start` alongside the existing fields:

```rust
let mut runtime = Runtime {
    // ... existing fields ...
    peer_authority_index: BTreeMap::new(),
};
```

### 3.2 `record_peer_authority` + `lookup_peer_for_author` — `crates/kernel/src/runtime.rs`

```rust
/// Bounded count of peers tracked per author in the
/// peer-authority index. Older entries are evicted on overflow.
/// Per B-4.6 spec §2 (decision table).
const PEER_AUTHORITY_PER_AUTHOR_CAP: usize = 8;

impl Runtime {
    /// Record that `peer` is known to have authority over `author`
    /// — we just received a signed HeadsSummary from `peer` advertising
    /// it. Move `peer` to the front of the per-author Vec (MRU); on
    /// overflow, drop the tail. Per B-4.6 spec §3.2.
    fn record_peer_authority(&mut self, peer: PeerPubkey, author: AuthorPubkey) {
        let entry = self.peer_authority_index.entry(author).or_default();
        // Remove existing occurrence (if any) so the move-to-front is
        // a single push at the head.
        entry.retain(|p| *p != peer);
        entry.insert(0, peer);
        if entry.len() > PEER_AUTHORITY_PER_AUTHOR_CAP {
            entry.truncate(PEER_AUTHORITY_PER_AUTHOR_CAP);
        }
    }

    /// Look up the most-recently-observed peer with authority over
    /// `author`. Returns `None` if we have never seen a HeadsSummary
    /// advertising this author.
    fn lookup_peer_for_author(&self, author: &AuthorPubkey) -> Option<PeerPubkey> {
        self.peer_authority_index
            .get(author)
            .and_then(|peers| peers.first().copied())
    }
}
```

### 3.3 Populate the index in `handle_heads_summary` — `crates/kernel/src/runtime.rs`

Currently `handle_heads_summary` (post-B-4.5, around `runtime.rs:1178`) starts with:

```rust
async fn handle_heads_summary(&mut self, remote: HeadsSummary) -> Result<(), RuntimeError> {
    self.check_fuel_table_version(&remote);
    let local_map: BTreeMap<...> = self.dag.author_heads().iter().map(...).collect();
    // ... diff logic ...
}
```

Insert the index population AFTER `check_fuel_table_version` but BEFORE building `local_map`:

```rust
async fn handle_heads_summary(&mut self, remote: HeadsSummary) -> Result<(), RuntimeError> {
    self.check_fuel_table_version(&remote);

    // B-4.6: populate the peer-authority index from the signed
    // HeadsSummary. The signer attests to having events for every
    // author in `remote.authors` (else it could not advertise valid
    // tip hashes). Future Pending/InvalidChain recoveries on these
    // authors will target `remote.signed_by_peer` via direct-stream.
    // Per B-4.6 spec §3.3.
    for head in &remote.authors {
        self.record_peer_authority(remote.signed_by_peer, head.author);
    }

    let local_map: BTreeMap<...> = ...;
    // ... rest of diff logic unchanged ...
}
```

### 3.4 `request_author_chain_gap` switchover — `crates/kernel/src/runtime.rs`

Currently (post-B-4.5, `runtime.rs:1111-1127`):

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
    let Ok(req) = self.build_signed_heads_request(requests) else {
        return;
    };
    let _ = self
        .network
        .publish(self.topic, GossipMessage::HeadsRequest(req))
        .await;
}
```

Replace the publish block with index-lookup + direct-stream + fallback:

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

    // B-4.6: prefer direct-stream to a peer known to have authority
    // over this author. Falls back to gossip-routed broadcast if the
    // index has not yet observed a peer for `author` — the legacy
    // recovery path remains as a safety net until B-4.7 retires it.
    if let Some(target_peer) = self.lookup_peer_for_author(&author) {
        self.issue_direct_backfill(target_peer, requests).await;
        return;
    }

    // Fallback: gossip-routed broadcast. Reached when this runtime
    // has never observed a HeadsSummary covering `author` (typically
    // a fresh-joiner mid-stream case). B-4.7 will remove the
    // fallback once index population is reliable.
    let Ok(req) = self.build_signed_heads_request(requests) else {
        return;
    };
    let _ = self
        .network
        .publish(self.topic, GossipMessage::HeadsRequest(req))
        .await;
}
```

### 3.5 Optional: `PeerWarning::EmptyPeerAuthorityIndex` — deferred

Per the decision table: when the index is empty for `author`, the runtime falls back silently to the gossip path. No new warning variant is added in B-4.6.

A future tuning step could add `PeerWarning::PeerAuthorityIndexEmpty { author }` to surface "we tried to back-fill but had no candidate peer" for observability. Deferred.

## 4. Acceptance tests

### 4.1 New file: `crates/kernel/tests/peer_authority_index.rs`

**Test 1: `index_populated_by_heads_summary_receipt`**

- Two peers A + B. A publishes a real signed HeadsSummary covering author X (X = AuthorPubkey::deterministic(42).author).
- B receives via gossip; `handle_heads_summary` populates B's index.
- Assert: B's index has `[A]` for X. Verify via a test-only `runtime_b.peek_peer_authority_for(&X)` shim, or via runtime introspection added behind `#[cfg(feature = "test-helpers")]`. **Decision: expose `lookup_peer_for_author` on `RuntimeHandle` behind `#[cfg(any(test, feature = "test-helpers"))]`** — minimal surface, test-only.

  Actually simpler: assert observable behavior. Inject a Pending event for X; verify B issues direct-stream to A (capturing-handler on A's MemNetwork). This is the structural assertion.

**Test 2: `index_move_to_front_on_repeated_observation`**

- Two peers A, B both publish HeadsSummary covering author X.
- C observes A's first, then B's, then A's again.
- C issues a Pending recovery for X (forge a Pending Event).
- Assert: C targets A (most-recently-observed) via direct-stream.

**Test 3: `index_caps_at_8_peers_per_author`**

- 9 peers each publish HeadsSummary covering author X.
- C observes all 9.
- Assert: C's peer-authority lookup for X picks ONE of the 9 (deterministic = most-recently-observed). Indirectly verify the cap by observing that C's behavior remains bounded (memory-wise, but this is hard to assert; primary assertion is just "lookup returns Some").

**Test 4: `pending_event_with_known_author_uses_direct_stream`**

- Peer A authors events 1, 2, 3. A's HeadsSummary populates B's index for A's author.
- B receives event 3 via gossip BEFORE events 1+2 (out-of-order delivery — forge via direct publish + selective subscribe).
- B's handle_event detects Pending → request_missing_for → request_author_chain_gap(A.author, 1, 2).
- Assert: B targets A's peer via direct-stream (NOT gossip).
- B converges to A's digest.

**Test 5: `pending_event_with_unknown_author_falls_back_to_gossip`**

- Peer B alone. Receives a hand-forged Event from a never-seen author with seq=3.
- B's request_author_chain_gap fires; index has NO entry for that author.
- Assert: B falls back to gossip-routed `publish(GossipMessage::HeadsRequest(...))` — captured via a tap subscription.

**Test 6: `invalid_chain_uses_direct_stream_when_index_populated`**

- Peer A authors events 1, 2. B starts empty. A's HeadsSummary populates B's index.
- B receives event 2 directly (skipping 1) → handle_event returns InvalidChain.
- B's request_author_chain_gap(A.author, 1, 1) fires.
- Assert: B targets A via direct-stream.

### 4.2 Updates to existing tests

- `crates/kernel/tests/convergence.rs::pending_event_triggers_heads_request_not_heads_summary` (line ~541) currently asserts that the Pending path emits `GossipMessage::HeadsRequest`. **Under B-4.6, this test may change behavior**: if the runtime's peer-authority index is populated by an earlier HeadsSummary in the test setup, the Pending path will go direct-stream instead.

  **Diagnosis approach**: read the test setup. If the runtime observes a HeadsSummary BEFORE the Pending event, B-4.6 will use direct-stream → the test's assertion that the tap sees `GossipMessage::HeadsRequest` will fail. The fix is either:
  - (a) Restructure the test so no HeadsSummary precedes the Pending → empty index → fallback to gossip-routed → test passes.
  - (b) Update the test to assert direct-stream usage instead.

  **Pick (a)** for minimal change: the test's INTENT is "Pending fires gossip-routed HeadsRequest," and B-4.6's design says the fallback is still gossip-routed when index is empty. By ensuring the test setup has no preceding HeadsSummary, the test continues to validate the fallback path — which is the exact behavior B-4.7 will eventually remove.

  Document the test update inline with a comment citing this rationale.

### 4.3 `RuntimeHandle::peek_peer_authority_index_for` test affordance — deferred

Tests assert behavior via direct-stream capture (point a CapturingRequestHandler at the target peer and observe that an inbound request arrives). No direct index introspection is needed.

If a future test requires it, expose under `#[cfg(any(test, feature = "test-helpers"))]` only.

## 5. Justfile changes

`just spec-coverage` regeneration after B-4.6 lands.

## 6. Edge cases

| Scenario | Behavior |
|---|---|
| HeadsSummary advertises 0 authors | The `for head in &remote.authors` loop is a no-op; index unchanged. Diff logic still runs (empty diff). |
| HeadsSummary advertises 100 authors | All 100 inserted into the index under their respective AuthorPubkey keys. Memory: 100 * (~32 + Vec overhead) ≈ 4 KiB worst case for first-observation. |
| Same peer publishes HeadsSummary twice for same author | First observation: insert. Second: move-to-front (no change since it's already at front). No duplicate. |
| Two peers race to publish HeadsSummary for same author | Both inserted; whichever was observed second is at position 0. Subsequent recovery picks the second. |
| Author becomes silent (no more HeadsSummaries) | Their entry stays in the index indefinitely. Recovery targets a stale peer until that peer fails (DirectRequestFailed warning), then next recovery picks a different peer or falls back to gossip. Manual cleanup not in scope. |
| Loopback HeadsSummary | The verify-side filter at `runtime.rs:1836` returns `false` for self-signed; `handle_heads_summary` is not called; index not populated with ourselves. Correct — we never need to back-fill from ourselves. |
| All known peers for an author become unreachable | `issue_direct_backfill` fails for each, logging DirectRequestFailed. Subsequent retries still target the same peer (no eviction). Recovery semantically fails until either (a) a new HeadsSummary observes a reachable peer or (b) the next periodic HeadsSummary tick on this runtime triggers a re-issue that succeeds. |
| Index empty for a needed author + gossip fallback also has no responders | Same as today: the recovery attempt is broadcast and ignored. Convergence stalls until some peer publishes new events. |

## 7. Surface change summary

**New crate-public surface**:

- `myrhiza_kernel::runtime::Runtime::peer_authority_index` field — private; not exposed.
- `myrhiza_kernel::runtime::Runtime::record_peer_authority`, `Runtime::lookup_peer_for_author` — `&mut self` / `&self` private methods; not exposed.
- `PEER_AUTHORITY_PER_AUTHOR_CAP` constant — `pub(crate)`; not exposed.

**No SemVer-breaking changes**.

**Unchanged**:
- `Network` trait surface.
- `RuntimeHandle` shape.
- `GossipMessage::HeadsRequest` wire variant.
- `handle_heads_request` gossip-routed inbound handler.
- `build_signed_heads_request` (still called by `request_author_chain_gap` fallback path).

## 8. Non-goals (explicit)

- **Removal of the gossip-routed inbound `handle_heads_request`.** B-4.7.
- **Removal of `GossipMessage::HeadsRequest` wire variant.** B-4.7. Wire-freeze regen.
- **Removal of the gossip-routed fallback in `request_author_chain_gap`.** B-4.7. Removes the `Ok(req) = self.build_signed_heads_request(requests) else { return };` block + the `publish(GossipMessage::HeadsRequest(...))` call.
- **Eviction-on-DirectRequestFailed for the peer-authority index.** Defer.
- **`PeerWarning::PeerAuthorityIndexEmpty` observability variant.** Defer.
- **Cross-process / cross-machine kernel-tier tests.** Defer.
- **Per-requester rate limiting at the responder.** Defer.
- **Discovery / pkarr / DHT integration.** Defer.

## 9. Prior-art consultation

- [`prior-art/iroh/gossip.md`](../prior-art/iroh/gossip.md) §"Topic IDs are a flat namespace with no auth" — confirms the protocol layer (iroh-gossip) has no authority concept; we layer our own.
- [`prior-art/iroh/docs.md`](../prior-art/iroh/docs.md) §"Authority" — iroh-docs uses NamespaceId for authority (closed membership). Myrhiza's open/capability-discriminated model means we don't inherit iroh-docs' authority shape; the peer-authority index is a Myrhiza-specific construct.
- [`docs/specs/2026-05-20-plan-b-4-2-attribution-design.md`](2026-05-20-plan-b-4-2-attribution-design.md) §3.0 — `HeadsSummary::signed_by_peer` field is THE source of attested peer-authority. Without B-4.2's signed HeadsSummary, the index would have no trustworthy source.
- [`docs/specs/2026-05-21-plan-b-4-5-kernel-runtime-integration-design.md`](2026-05-21-plan-b-4-5-kernel-runtime-integration-design.md) §3.6 — `issue_direct_backfill(target_peer, requests)` is the existing helper this slice piggy-backs on.

**Gaps**: none new — the existing direct-stream surface from B-4.4/4.5 supports everything B-4.6 needs.

## 10. Future work — explicit deferrals

- **B-4.7** — Retire the gossip-routed surface:
  - Remove `handle_heads_request` (the inbound dispatch).
  - Remove `build_signed_heads_request` (no remaining callers).
  - Remove the gossip-routed fallback in `request_author_chain_gap`.
  - Remove or deprecate `GossipMessage::HeadsRequest` variant — wire-freeze regen.
- **Eviction-on-failure for peer-authority index.** Track DirectRequestFailed counts per peer; demote/evict above a threshold.
- **`PeerWarning::PeerAuthorityIndexEmpty` observability.**
- **Cross-process / cross-machine tests.**
- **Per-requester rate limiting at the responder.**
- **Discovery / pkarr / DHT integration.**
- **`Lagged`-event mapping test on real iroh-gossip** (B-4.1 §4 carryover).
- **`PeerWarning::SignatureInvalid` backfill in `process_drift_message`** (B-4.2 §10 carryover).
- **`HeadsStreamError::Handler` variant population** (B-4.4 final-review reservation).

## 11. Sources

- `crates/kernel/src/runtime.rs:1084-1102` — `request_missing_for` (Pending caller).
- `crates/kernel/src/runtime.rs:1111-1127` — `request_author_chain_gap` (modified in §3.4).
- `crates/kernel/src/runtime.rs:1048-1056` — `InvalidChain` arm in `handle_event` (second caller of `request_author_chain_gap`).
- `crates/kernel/src/runtime.rs:1178+` — `handle_heads_summary` (modified in §3.3).
- `crates/kernel/src/runtime.rs:1150+` — `issue_direct_backfill` (B-4.5 helper, reused).
- `crates/kernel/src/runtime.rs:1822-1840` — `verify_heads_summary` loopback filter (cited in §6 edge cases).
- `crates/kernel/tests/convergence.rs:541` — Pending-path test that may need restructuring (per §4.2).
- [`docs/specs/2026-05-20-plan-b-4-2-attribution-design.md`](2026-05-20-plan-b-4-2-attribution-design.md).
- [`docs/specs/2026-05-21-plan-b-4-5-kernel-runtime-integration-design.md`](2026-05-21-plan-b-4-5-kernel-runtime-integration-design.md).
- [`prior-art/iroh/gossip.md`](../prior-art/iroh/gossip.md).
