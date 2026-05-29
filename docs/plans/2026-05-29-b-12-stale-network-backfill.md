**Date:** 2026-05-29
**Status:** active
**Spec:** [B-12 stale-network backfill design](../specs/2026-05-29-b-12-stale-network-backfill-design.md)
**Subject:** Implement on-start + periodic distribution-log catch-up + staleness surface.

# Plan B-12 — Stale-network backfill for revocation/publication logs

## Preamble

Worktree: `/mnt/storage/projects/myrhiza/.claude/worktrees/b-10-revocation-wiring`
Branch: `feat/b-12-stale-network-backfill` (stacked on `feat/b-11-revocation-subscription`; PR #29 not yet merged). Base for the eventual PR is the B-11 branch (or `main` once #29 lands).

Each task is a buildable, test-green increment (TDD: RED test first, then GREEN
code). Run the per-task verification before moving on. Follow CLAUDE.md: quality +
longevity over speed; root-cause every failure; zero clippy warnings; never
`--no-verify`; never relax a test to make it pass.

**WORKTREE CAVEAT:** do **not** run `just build-fixtures` / `just build-fixtures-check`
here — they fail in a worktree (excluded standalone fixture crates bind to the
*main* repo workspace). The committed `tests/fixtures/built/*.wasm` satisfy all
kernel-tier tests. Use `cargo test` / `cargo check` directly. The canonical
`just ci` gate (which includes build-fixtures) runs from the primary checkout.

**Spec-coverage map** (every spec § lands in a task):

| Spec § | Task |
|---|---|
| §3.1 transport (gossip, no new Network method) | T3 |
| §3.2 summary variants 5/6 + wire-freeze | T1 |
| §3.3 archive in kernel | T2 |
| §3.4 push-driven symmetric convergence | T3, T4 |
| §3.5 on-start + periodic tick | T4 |
| §3.6 unsigned summaries + push rate-limit | T1, T3 |
| §3.7 staleness surface | T5 |
| §6 iroh-tier acceptance (closes distribution.md §10.7) | T6 |
| §6 lint/matrix/docs | T7 |

---

## Task T1 — `GossipMessage::{RevocationHeads, PublicationHeads}` + wire-freeze

**Spec ref:** §3.2, §3.6, §4.1.

**Subject:** Add the two append-only summary variants (discriminants 5/6) and the
payload structs; pin the wire tags.

**Resolve open question §12 Q1 first:** decide where `RevocationHeads` /
`PublicationHeads` live. Default: `crates/distribution` (alongside the event
envelopes), re-exported and depended-on by `crates/network` exactly as
`RevocationEvent`/`PublicationEvent` already are. **Verify no dependency cycle** —
B-11 inverted the `network ↔ distribution` edge (distribution must NOT depend on
network). `cargo run -p dep-direction-check` must stay green; `cargo tree` must
show no cycle.

**Files touched:**
- Create/modify: payload structs in `crates/distribution/src/` (e.g. a new
  `heads.rs`, or beside revocation/publication), `Serialize`/`Deserialize`,
  `Clone, Debug, PartialEq, Eq`, with `author: AuthorPubkey, last_observed_seq: u64`.
- Modify: `crates/network/src/lib.rs` — append `RevocationHeads(RevocationHeads)`
  (5) + `PublicationHeads(PublicationHeads)` (6) AFTER `Publication`(4). Do **not**
  reorder existing variants. Update the enum doc-comment wire-freeze note.
- Modify (RED first): `crates/types/tests/wire_freeze.rs` — add
  `gossip_message_revocation_heads_variant_tag_is_five_u32_be` (asserts the encoded
  discriminant is `[0,0,0,5]`) and `..._publication_heads..._six...` (`[0,0,0,6]`).
  Mirror the existing B-11 tag tests; use `AuthorPubkey`/`BlobHash` zero values.

**Verification:**
```bash
cargo test -p myrhiza-types --test wire_freeze
cargo check --workspace --all-targets
cargo run -p dep-direction-check --quiet
```

---

## Task T2 — Kernel event archive + population on apply

**Spec ref:** §3.3, §4.2, §4.3.

**Subject:** Store signed envelopes so a peer can serve backfill.

**Files touched:**
- Modify: `crates/kernel/src/runtime.rs` —
  - Add fields: `revocation_archive: BTreeMap<AuthorPubkey, BTreeMap<u64, RevocationEvent>>`,
    `publication_latest: BTreeMap<AuthorPubkey, PublicationEvent>`.
  - In `handle_revocation`, on apply-`Ok`, insert
    `revocation_archive.entry(author).or_default().insert(ev.revocation_seq, ev.clone())`.
    (`ev` is borrowed today — confirm it can be cloned here.)
  - In `handle_publication`, on apply-`Ok`, set
    `publication_latest.insert(author, ev.clone())` **before** `ev.version` is moved
    onto `PublicationAnnounced` (clone first, or reorder the move).
- RED first: `#[cfg(test)]` unit tests in `runtime.rs` (or `crates/kernel/tests/`):
  apply a valid revocation → `revocation_archive[author][seq]` holds the envelope;
  apply two publications → `publication_latest[author]` holds the newer one.

**Verification:**
```bash
cargo test -p myrhiza-kernel
cargo check -p myrhiza-kernel   # iroh-free path still compiles
```

---

## Task T3 — Summary receive → push-on-behind + rate-limit

**Spec ref:** §3.1, §3.4, §3.6, §4.3, §4.5 (arm 6 dispatch).

**Subject:** Make `handle_distribution_message` async; on a below-our-head summary,
re-publish the delta (revocation: contiguous range from archive; publication:
latest envelope), gated by a per-author token bucket.

**Files touched:**
- Modify: `crates/kernel/src/runtime.rs` —
  - Add field `distribution_push_limit: BTreeMap<AuthorPubkey, DriftRateLimit>`
    (reuse the existing `DriftRateLimit`; pick capacity/refill from §12 Q3 — start
    at DriftRateLimit defaults).
  - `handle_distribution_message` → `async fn`; the sixth select arm awaits it
    (`self.handle_distribution_message(author, msg).await`).
  - Add `async fn handle_revocation_heads(&mut self, author, remote_seq)`:
    bump `last_distribution_sync` (T5 introduces the field — add a stub here or
    sequence T5 before pushing the sync-clock bump; simplest: add the field in T3
    and surface it in T5). `local = revocation_logs[author].last_observed_seq`; if
    `remote_seq < local` and the bucket admits, for `seq in remote_seq+1..=local`
    `network.publish(Topic::from_bytes(derive_revocation_topic(author)),
    GossipMessage::Revocation(archive[author][seq].clone()))`.
  - Add `async fn handle_publication_heads(...)`: if `remote_seq < local` and bucket
    admits, push `GossipMessage::Publication(publication_latest[author].clone())`.
  - Summary with mismatched carried `author` → `PeerWarning::DecodeFailed`.
- RED first: MemNetwork kernel-tier test — peer A holds revocations seq 1..3 +
  archive; feed A a `RevocationHeads{author, 0}` (as if from a behind peer);
  assert A publishes 3 `Revocation` messages (observe via a MemNetwork capture /
  second subscribed peer). Add a rate-limit test: a burst of low summaries yields
  a bounded push count.

**Verification:**
```bash
cargo test -p myrhiza-kernel
cargo clippy -p myrhiza-kernel --all-targets -- -D warnings
```

---

## Task T4 — On-start + periodic broadcast (headline catch-up)

**Spec ref:** §3.4, §3.5, §4.4, §4.5 (seventh arm), §4.6.

**Subject:** Advertise our heads on start and on a slow tick, so a rejoining peer
triggers ahead-peers to push.

**Files touched:**
- Modify: `crates/kernel/src/runtime.rs` —
  - `async fn broadcast_distribution_heads(&mut self)`: for each installed author,
    `publish(revocation_topic, RevocationHeads{author, rev_last_seq})` and
    `publish(publication_topic, PublicationHeads{author, pub_last_seq})` (0 if no
    log). Track installed authors — `Runtime::start` already receives
    `installed_authors`; stash them on the struct if not already.
  - Call `broadcast_distribution_heads` once after
    `subscribe_distribution_topics` in `Runtime::start`/`run`.
  - Add `distribution_sync_tick` config (default 30 s) + a seventh select arm
    `_ = dist_ticker.tick() => self.broadcast_distribution_heads().await?;`.
- RED first (**the B-12 acceptance test**): two MemNetwork peers, author A.
  Peer C applies revocation seq 1. Peer B starts with an empty log (installed
  author A). After B's on-start broadcast, B's `revocation_events` reports the
  revocation and `revoked_bundles == {X}`. Use `condition-based-waiting`
  (`poll_until_nonempty`), not a fixed sleep.

**Verification:**
```bash
cargo test -p myrhiza-kernel
cargo test --workspace --all-targets
```

---

## Task T5 — Staleness surface (`last_distribution_sync` + `stale_authors`)

**Spec ref:** §3.7, §4.2.

**Subject:** Track per-author last-sync time; expose it + a 24 h helper.

**Files touched:**
- Modify: `crates/kernel/src/runtime.rs` —
  - Add `last_distribution_sync: Arc<Mutex<BTreeMap<AuthorPubkey, SystemTime>>>`;
    write `SystemTime::now()` on every received distribution message (event or
    summary) in `handle_distribution_message` paths.
  - `RuntimeHandle` gains `last_distribution_sync: Arc<Mutex<…>>` (clone of the
    Arc) + `pub fn stale_authors(&self, now: SystemTime, threshold: Duration,
    installed: &[AuthorPubkey]) -> Vec<AuthorPubkey>` returning authors whose last
    sync is `None` or older than `now - threshold`. (Take `now` as a param per
    §12 Q5 for deterministic tests.)
- Modify: `crates/test-utils` `PeerHandle` — add a `stale_authors`/sync accessor if
  the harness needs to assert it.
- RED first: unit test — never-synced author is stale; an author synced `now` is
  not; an author synced `now - 25h` is stale at a 24 h threshold.

**Verification:**
```bash
cargo test -p myrhiza-kernel -p myrhiza-test-utils
```

---

## Task T6 — Iroh-tier acceptance (closes `distribution.md` §10.7)

**Spec ref:** §6 iroh tier.

**Subject:** Prove real late-join catch-up over `iroh-gossip`.

**Files touched:**
- Create: `crates/kernel/tests/iroh_stale_backfill.rs`
  (`#![cfg(feature = "network-iroh")]`). Author A (deterministic signing key);
  publisher peer broadcasts a revocation on A's topic; a second peer that *was not
  subscribed at broadcast time* then starts with `installed_authors=[A]`, runs its
  on-start broadcast, and catches up. Assert its `revocation_events` reflects the
  revocation. Bounded poll loop with timeout; reuse `IrohHarness::spawn_peer`
  (gained `installed_authors` in B-11) and the raw `spawn_iroh_peer` for the
  publisher. Run 3× to confirm non-flaky; record observed settle timing in a
  comment.

**Verification:**
```bash
cargo test -p myrhiza-kernel --features network-iroh --test iroh_stale_backfill
# run 3x to confirm stability
```

---

## Task T7 — Lint + matrix + docs

**Spec ref:** §6 (not-tested boundary), §7, §8.

**Subject:** Final verification + doc updates.

**Files touched:**
- Modify: `docs/README.md` — add B-12 catalog entries under **Runtime core** and
  **App distribution** (mirror the B-11 dual entry; `[active]` until landed).
- Modify: `docs/reports/2026-05-21-mvp-gap-analysis.md` — dated 2026-05-29 note
  that the `distribution.md` §10.7 stale-network mitigation is now implemented by
  B-12.
- Modify: `tests/spec-coverage.md` if the coverage script tracks the new spec
  (run `just spec-coverage` from the primary checkout if needed; in-worktree, note
  it as a primary-checkout step).

**Verification (full gate, both feature sets):**
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --all-targets --features network-iroh -- -D warnings
cargo test --workspace --all-targets
cargo test -p myrhiza-network -p myrhiza-kernel -p myrhiza-test-utils --features network-iroh --tests
cargo run -p dep-direction-check --quiet
```

---

## Done criteria

- All 7 tasks green; full dual-feature gate passes.
- Wire-freeze: discriminants 0–4 unchanged; 5/6 pinned.
- No scope drift into deferred items (direct-stream, signed summaries, durable
  archive/B-9, install-time gate, >1024-behind publication).
- `iroh_stale_backfill.rs` proves real late-join catch-up (closes
  `distribution.md` §10.7).
