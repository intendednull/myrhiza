**Date:** 2026-05-29
**Status:** active
**Subject:** B-12 — stale-network backfill for revocation/publication logs (close `distribution.md` §10.7)

# B-12 — Stale-network backfill for revocation/publication logs

## §1 Goal

B-11 wired the `crates/distribution` pure tier into the kernel `Runtime`: an
installed author's revocation + publication topics are auto-subscribed, and
inbound gossip flows verify → apply → surface. But the mechanism is **gossip-only
and online-only**: a peer that is offline (or partitioned, or simply not yet
joined) while an author broadcasts a revocation **never learns about it**. Missed
gossip is missed forever. This is precisely the *stale-network attack* the master
design calls out:

> **Stale-network attack**: an adversary may withhold revocation events from a
> target peer. Mitigation: revocation topic is part of the auto-subscribed set;
> peers run a HeadsSummary-shape sync on the revocation topic at start to backfill
> missed revocations. Peers without a fresh sync within 24 hours surface a
> "potentially stale" warning before installing a new version.
> — [`distribution.md` §10.7](2026-05-09-myrhiza-master-design/distribution.md)

B-12 closes that gap. After this slice, a peer that rejoins (or starts cold)
**catches up the full revocation set and the latest publication** for every
installed author by exchanging compact log-head summaries over the existing
per-author distribution topics — and surfaces a staleness signal when it has not
managed a fresh sync.

This is the backfill counterpart to the event-DAG's B-1 `HeadsSummary` +
B-4.4 direct-stream backfill, scaled down to the much simpler shape of the
distribution logs (linear per-author sequence, no DAG, no deps, tiny payloads).

## §2 Scope

### In v1 (this slice)

- Two new **append-only** `GossipMessage` summary variants —
  `RevocationHeads`(5) / `PublicationHeads`(6) — advertising a peer's
  `last_observed_seq` for an author's log, broadcast on the existing per-author
  derived topics. Wire-freeze preserved (append after B-11's 3/4).
- A **kernel-side event archive** of the signed envelopes (`revocation_archive`
  full per-author; `publication_latest` latest-only) so a peer can *serve*
  backfill — the folded `RevocationLog`/`PublicationLog` state alone cannot
  (signatures are unreconstructable).
- **Push-on-behind** catch-up: on receiving a summary whose seq is *below* ours,
  re-publish the missing signed events (revocation: contiguous range from the
  archive; publication: the single latest envelope) on the topic, reusing the
  existing `Revocation`(3)/`Publication`(4) variants. Idempotent — the receiver's
  monotonic-seq check drops duplicates.
- **On-start + periodic** summary broadcast (a `distribution_sync_tick`) so a
  rejoining peer advertises its low head and any ahead-peer pushes the delta.
- **Amplification rate-limit**: a per-author token bucket caps how often a peer
  will re-broadcast a delta, so a flood of forged low summaries cannot weaponise
  ahead-peers.
- **Staleness surface**: per-author `last_sync_at` tracking + a
  `RuntimeHandle::stale_authors(now, threshold)` helper (default threshold 24 h)
  exposing which installed authors have not had a fresh sync.
- Kernel-tier (MemNetwork) acceptance tests + an iroh-tier test proving real
  late-join catch-up over `iroh-gossip`.

### Explicitly deferred

- ~~**Direct-stream (request/response) backfill** for the distribution logs.~~
  **DONE — promoted in §14.** Originally deferred here (runner-up to §3.1,
  rejected for v1 on the assumption gossip re-broadcast was adequate). The §13
  finding proved gossip-push cannot catch up a late joiner over real iroh-gossip,
  so the direct-stream pull was promoted and is the landed transport (§14): a new
  `request_distribution` protocol mirroring the `request_heads` rails.
- **Signed summaries.** v1 summaries are unsigned (§3.6/§14.1 rationale: they only
  ever trigger the hearer to *pull* *already author-signed* events it then
  verifies; the worst a forged-high summary buys is one wasted dial, capped by the
  per-advertiser dial-limit). Signing the summary (HeadsSummary-style
  `signed_by_peer`) is a future hardening. → future.
- **Durable persistence of the archive / logs across restart.** The archive is
  RAM-only this slice; it repopulates from the network on the next sync. Durable
  storage + GC of the archive is **B-9** territory. → B-9.
- **Install-time gate enforcement.** B-12 *surfaces* the staleness signal; the
  policy "block / warn before installing a new version when stale" is consumed by
  the install flow, which is a separate surface. B-12 delivers the signal, not the
  gate. → install-flow follow-on.
- **Publication catch-up for peers >1024 releases behind.** Publication is
  latest-wins and backfilled head-only; a peer more than `MAX_PUBLICATION_JUMP`
  behind cannot be caught by a single head push. This is an extreme edge (1024
  releases) and is left to the future durable-log/snapshot path. Revocation has no
  such limit (contiguous push never trips the cap). → future.

## §3 Design decisions

> **⚠️ §3.1 (transport = gossip re-broadcast) and §3.4 (push-on-behind) are
> SUPERSEDED by §14.** Implementation (the iroh-tier test) proved gossip
> re-broadcast cannot catch up a late joiner over real iroh-gossip — see the §13
> finding and the §14 corrected design (direct-stream pull). The remaining §3
> decisions (§3.2 summary variants, §3.3 archive, §3.5 advertise trigger,
> §3.6→partial, §3.7 staleness surface) still hold. Read §13 + §14 before §3.1/§3.4.

*Locked during design from the B-11/B-10 prior-art + master-spec consultation,
2026-05-29. Each decision names the runner-up and why it was rejected.*

### §3.1 Transport: gossip re-broadcast, not direct-stream

**Decision.** Catch-up rides the existing topic `subscribe`/`publish` gossip
path. A behind-peer advertises a summary; an ahead-peer *pushes* the missing
signed events by re-`publish`-ing them on the same per-author topic. No new
`Network` trait method.

**Runner-up: direct-stream request/response** (mirror B-4.4 `request_heads` /
`HeadsStream` with new `DirectRevocationRequest` types). *Rejected* because (a) it
widens the `Network` ABI for messages that are ~100 bytes and emitted a handful of
times per author *ever* — the direct-stream machinery exists to keep *large*
DAG-event payloads off gossip, a cost the distribution logs do not have; (b) B-11
deliberately rode `subscribe`/`publish` for the distribution tier rather than
widening `Network`, and B-12 should not reverse that; (c) the master spec frames
the mitigation as a *"HeadsSummary-shape sync on the revocation topic"* — i.e. on
the topic, via gossip. Validated by the event-DAG precedent, which *also* uses
gossip push for the "Ahead" case ([`runtime.rs` `AuthorDiff::Ahead`]) and reserves
direct-stream only for large pulls.

### §3.2 Summary message: new append-only `GossipMessage` variants (5/6)

**Decision.** Add `RevocationHeads { author, last_observed_seq }` (discriminant 5)
and `PublicationHeads { author, last_observed_seq }` (discriminant 6). One variant
per topic family, matching the two-topic structure. `author` is carried (not just
implied by the topic) so a misrouted summary is detectable, symmetric with how
B-11 defensively maps misrouted `Revocation`/`Publication` to a `PeerWarning`.
Wire-freeze: append-only after B-11's 3/4; new `wire_freeze.rs` tests pin the
u32-BE tags `[0,0,0,5]` / `[0,0,0,6]`.

**Runner-up: extend the app-topic `HeadsSummary`** with distribution fields.
*Rejected* — that couples the event-DAG sync (one shared app topic) to the
distribution logs (per-author derived topics); the summary would advertise state
for a topic it is not broadcast on, and a peer subscribed to the app topic but not
to an author's revocation topic would receive irrelevant heads. The master spec
explicitly scopes the sync to *the revocation topic*.

**Runner-up: a single unified `DistributionHeads { author, rev_seq, pub_seq }`**
broadcast on both topics. *Rejected* — half the payload is irrelevant on each
topic, and it muddies the per-topic handler. Two narrow variants are cleaner.

### §3.3 Event archive lives in the kernel, pure tier unchanged

**Decision.** The signed-envelope archive is kernel `Runtime` state, not part of
the pure-tier `RevocationLog`/`PublicationLog`. Revocation keeps a full
`revocation_archive: BTreeMap<AuthorPubkey, BTreeMap<u64, RevocationEvent>>`;
publication keeps `publication_latest: BTreeMap<AuthorPubkey, PublicationEvent>`
(latest-wins ⇒ one envelope suffices to serve a backfill). Both are populated in
the existing `handle_revocation`/`handle_publication` on a successful apply, so
gossip-received and backfill-received events archive uniformly.

**Runner-up: fold the archive into the pure-tier log structs.** *Rejected* —
backfill-serving is a networking concern, not a state-machine concern. The pure
tier stays a minimal deterministic fold (set + seq / latest + seq); bloating it
with raw signed events would (a) enlarge what B-9 must persist as "log state" and
(b) blur the determinism boundary. Keep the fold pure; keep the archive in the
kernel beside the existing `revocation_logs` map.

**Why revocation full vs publication latest-only:** revocation accumulates (every
event contributes a distinct `revoked_bundle_hash` to the set), so a complete set
needs every event in range; publication is latest-wins, so the newest envelope
reconstructs the entire observable state. This asymmetry mirrors the log
semantics, not an arbitrary optimisation.

### §3.4 Convergence is push-driven and symmetric

**Decision.** No peer issues an explicit "send me X" request. Each peer
periodically (and on start) broadcasts its own head summary. A peer that hears a
summary *below* its own head responds by pushing the delta; a peer that hears one
*above* its own head simply records the contact (it will receive the pushed delta,
and its own next summary will prompt the ahead-peer). Convergence is the fixpoint
of "everyone advertises, ahead-peers push." This is the same shape as the
event-DAG's advertise-then-push/pull loop, minus the pull (direct-stream) half.

**Runner-up: explicit pull** (behind-peer sends a request variant). *Rejected* —
adds a request variant and a requester-identity story that gossip does not give
cleanly; the symmetric push covers the same ground with one fewer message type.

### §3.5 Trigger: on-start broadcast + a dedicated periodic tick

**Decision.** After `subscribe_distribution_topics` returns, broadcast an initial
`RevocationHeads`/`PublicationHeads` for each installed author (this is the
"sync on start" the master spec names). Add a `distribution_sync_tick`
(default 30 s — distribution changes are far rarer than DAG events, so it need not
match the 5 s `heads_summary_tick`) that re-broadcasts summaries, recovering from
transient partitions and `SubError::Lagged` on the distribution subscriptions.

**Runner-up: reuse `heads_summary_tick` (5 s).** *Rejected* — needlessly chatty
for logs that change a handful of times per author ever; a separate, slower tick
is cheaper and independently tunable.

### §3.6 Summaries are unsigned; pushes are rate-limited

**Decision.** Summary variants carry no signature. The security-critical artefacts
— the `RevocationEvent`/`PublicationEvent` envelopes — are author-signed and
verified at the `dispatch::verify_*` edge on apply, unchanged from B-11. A summary
can only ever *trigger* a re-broadcast of events the pusher already holds and the
receiver independently verifies. To prevent a forged-low-summary **amplification
flood**, each peer rate-limits its delta re-broadcasts with a per-author token
bucket (`distribution_push_limit`, reusing the `DriftRateLimit` shape). A burst of
forged summaries is absorbed by the bucket.

**Runner-up: sign summaries** (`signed_by_peer` like `HeadsSummary`). *Rejected
for v1* — it adds a signature surface and verification path to defend against an
attack whose worst outcome (with the rate-limit in place) is bounded redundant
gossip. Promotable later if the threat model tightens.

### §3.7 Staleness surface, not staleness gate

**Decision.** Track `last_sync_at: BTreeMap<AuthorPubkey, SystemTime>`, updated
whenever *any* distribution message (summary or event) for an author is received
— evidence the topic is reachable. Expose it via the `RuntimeHandle` (poll-log
pattern, twin of `revocation_events`) plus a helper
`stale_authors(now: SystemTime, threshold: Duration) -> Vec<AuthorPubkey>`
returning authors whose last sync is older than `threshold` (or never synced).
Default threshold 24 h per the master spec. Wall-clock lives only in the kernel
orchestration task (never in any deterministic state-apply path), consistent with
the existing `heads_summary_tick` interval.

**Runner-up: emit a `PeerWarning::DistributionStale` on a timer.** *Rejected* —
staleness is a question the *consumer* (install flow / UI) asks at decision time
("am I about to install while stale?"), not a continuous background event; a
pollable helper fits the consumer better than a log entry it would have to dedupe.

## §4 Design

### §4.1 Wire format

```rust
// crates/network/src/lib.rs — appended to GossipMessage (append-only)
pub enum GossipMessage {
    Event(Event),                 // 0  (frozen)
    HeadsSummary(HeadsSummary),   // 1  (frozen)
    Drift(DriftMessage),          // 2  (frozen)
    Revocation(RevocationEvent),  // 3  (B-11)
    Publication(PublicationEvent),// 4  (B-11)
    RevocationHeads(RevocationHeads),   // 5  (B-12)  NEW
    PublicationHeads(PublicationHeads), // 6  (B-12)  NEW
}

// New summary payloads (crates/distribution — beside the event envelopes,
// or crates/network if they must avoid a dep; see §12 Q1).
pub struct RevocationHeads {
    pub author: AuthorPubkey,
    pub advertiser: PeerPubkey, // who is advertising — loopback filter (§3.2) + §13 pull-dial target
    pub last_observed_seq: u64,
}
pub struct PublicationHeads {
    pub author: AuthorPubkey,
    pub advertiser: PeerPubkey,
    pub last_observed_seq: u64,
}
```

Encoded with the v1 canonical-bincode options (u32-BE fixint variant tags), pinned
by `crates/types/tests/wire_freeze.rs`.

### §4.2 Kernel state (additions to `Runtime`)

```rust
// archive of signed envelopes, to serve backfill (§3.3)
revocation_archive: BTreeMap<AuthorPubkey, BTreeMap<u64, RevocationEvent>>,
publication_latest: BTreeMap<AuthorPubkey, PublicationEvent>,
// amplification guard (§3.6) — one bucket per author
distribution_push_limit: BTreeMap<AuthorPubkey, DriftRateLimit>,
// staleness surface (§3.7) — Arc so RuntimeHandle can poll it
last_distribution_sync: Arc<Mutex<BTreeMap<AuthorPubkey, SystemTime>>>,
```

`RuntimeHandle` gains `last_distribution_sync: Arc<Mutex<…>>` plus the
`stale_authors(now, threshold)` helper.

### §4.3 Receive path (extends B-11's `handle_distribution_message`)

`handle_distribution_message` becomes `async` (it may `publish`), dispatching:

- `Revocation(ev)` → `handle_revocation` (unchanged) **+** archive insert
  `revocation_archive[author][ev.revocation_seq] = ev` on apply-Ok; bump
  `last_distribution_sync[author]`.
- `Publication(ev)` → `handle_publication` (unchanged) **+**
  `publication_latest[author] = ev` on apply-Ok; bump sync clock.
- `RevocationHeads { author, last_observed_seq: remote }` →
  `handle_revocation_heads`: bump sync clock; let `local =
  revocation_logs[author].last_observed_seq`; if `remote < local` and the per-author
  bucket admits, for `seq in remote+1..=local` push `archive[author][seq]` via
  `network.publish(derive_revocation_topic(author), Revocation(ev))`.
- `PublicationHeads { author, last_observed_seq: remote }` →
  `handle_publication_heads`: bump sync clock; if `remote < local` and bucket
  admits, push `publication_latest[author]` (single envelope).
- A summary whose carried `author` mismatches the subscription's author →
  `PeerWarning::DecodeFailed` (defensive, mirrors B-11).

### §4.4 Send path (on-start + tick)

A `broadcast_distribution_heads` helper iterates installed authors and publishes
`RevocationHeads`/`PublicationHeads` with the current `last_observed_seq` (0 if no
log yet) on each derived topic. Called once after `subscribe_distribution_topics`
in `Runtime::start`, and from a new `distribution_sync_tick` select arm.

### §4.5 Select loop

A **seventh arm** drives the periodic broadcast:

```rust
_ = dist_ticker.tick() => { self.broadcast_distribution_heads().await?; }
```

The existing sixth arm (`distribution_rx.recv()`) now dispatches to the `async`
`handle_distribution_message`. No change to arms 1–5.

### §4.6 Flow — late joiner catches up a missed revocation

1. Author A revokes bundle X (seq=1) while peer B is offline. Peer C (online)
   applies it and archives it.
2. B starts, `subscribe_distribution_topics` joins A's revocation topic, then
   `broadcast_distribution_heads` publishes `RevocationHeads{A, 0}`.
3. C receives the summary, sees `remote=0 < local=1`, bucket admits → pushes
   `Revocation(ev@seq1)`.
4. B receives it via the sixth arm → `handle_revocation` verifies + applies →
   `revoked_bundles = {X}`, archives it, bumps `last_distribution_sync[A]`.
5. B's `RuntimeHandle.revocation_events` now reports the revocation; `stale_authors`
   no longer lists A.

### §4.7 Dependencies / Cargo

No new external crates. The summary payload types live wherever they avoid a
dependency cycle (see §12 Q1) — most likely `crates/distribution` (already a
`crates/network` dep since B-11) re-exported, or `crates/network` directly if the
former cycles. `DriftRateLimit` is reused from the kernel.

## §5 Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Amplification via forged low summaries | Med | Gossip flood | Per-author token-bucket rate-limit on pushes (§3.6) |
| Archive unbounded RAM growth | Low | Memory | Revocations are rare; full archive is small. Durable + GC is B-9. Document. |
| Publication peer >1024 behind can't catch up via head push | Low | Stale publication view | Extreme edge (1024 releases); documented deferral. Revocation unaffected (contiguous push). |
| `handle_distribution_message` becoming async introduces a `.await` that stalls the loop | Low | Loop latency | Pushes are bounded by the rate-limit and small; mirror the existing async `handle_message` arm. |
| Wire-freeze regression (variant tag shift) | Low | Wire break | Append-only at 5/6; snapshot tests pin all of 0–6. |
| Determinism leak from wall-clock staleness | Low | Convergence bug | `last_sync_at` lives only in the kernel task, never in state-apply; same posture as `heads_summary_tick`. |

## §6 Test plan

### State / unit tier
- `revocation_archive` populated on apply (gossip + backfill paths).
- `publication_latest` holds newest envelope.
- Rate-limit bucket admits then denies a burst.
- `stale_authors` returns never-synced + past-threshold authors, excludes fresh.

### Kernel tier (MemNetwork)
- **Headline acceptance:** late joiner (empty log) catches up a revocation it
  missed, after `subscribe` + on-start summary triggers an ahead-peer push.
- Behind-peer catches up a *range* (seq 1..N) contiguously; final set == full set.
- Publication late joiner catches up the latest announcement.
- Forged low-summary flood is rate-limited (push count bounded).
- Misrouted summary (wrong author) → `PeerWarning::DecodeFailed`.
- Already-in-sync summary → no push (idempotent / no-op).

### Iroh tier (`#![cfg(feature = "network-iroh")]`)
- Real `iroh-gossip`: peer offline during a revocation, joins, catches up; assert
  `revocation_events` reflects it. Closes `distribution.md` §10.7. Run ≥3× for
  non-flakiness; record settle timing.

### Not tested this slice
- Durable archive across restart (B-9). Install-time staleness gate (follow-on).
  Signed summaries (deferred).

## §7 Surface change summary

> Updated for the §14 corrected transport (direct-stream pull). The original
> §3.1 "no new `Network` method" line is superseded: pull adds two additive
> `Network` methods.

- **New public:** `GossipMessage::{RevocationHeads, PublicationHeads}` + the two
  payload structs; `RuntimeHandle::stale_authors` + `last_distribution_sync`;
  the distribution-backfill direct-stream surface in `crates/network`
  (`Network::request_distribution` + `Network::install_distribution_handler`,
  `DistributionHandler`/`DistributionStream`/`DistributionResponder`,
  `DISTRIBUTION_REQUEST_ALPN`) and the wire types in `crates/distribution`
  (`DistributionBackfillRequest`, `DistributionEnvelope`, `DistributionLogKind`).
- **Modified:** `handle_distribution_message` → `async`; `Runtime::start` broadcasts
  initial summaries (signature unchanged — still takes `installed_authors`) and
  installs a `KernelDistributionHandler`; the heads handlers swap push→pull
  (per-author push rate-limit replaced by a per-advertiser dial-limit,
  `DISTRIBUTION_DIAL_DAILY_CAP`).
- **Additive ABI:** `Network` gains `request_distribution` +
  `install_distribution_handler` (out-of-tree impls must add them; both in-tree
  impls updated). The event-DAG `request_heads` protocol is **untouched** — the
  distribution protocol is a new, parallel direct-stream (§14.2).
- **Unchanged:** the pure-tier `RevocationLog`/`PublicationLog`; `dispatch::verify_*`;
  topic derivation; the `request_heads` backfill; discriminants 0–6.
- **Features:** iroh tests behind `network-iroh`, as B-11.

## §8 Cross-references

- Master: [`distribution.md` §10.7](2026-05-09-myrhiza-master-design/distribution.md)
  (scope contract), `determinism.md` §5 (state-apply purity — staleness clock stays
  out of it).
- Siblings: [B-10](2026-05-26-b-10-bundle-distribution-design.md) (distribution
  pure tier + topics), [B-11](2026-05-28-b-11-revocation-subscription-design.md)
  (the subscription wiring B-12 extends), B-1 / B-4.4 (the event-DAG
  `HeadsSummary` + direct-stream backfill this mirrors), **B-9** (durable archive).
- Report: [mvp gap analysis](../reports/2026-05-21-mvp-gap-analysis.md).

## §9 Prior-art consulted

- `prior-art/willow/` — per-author signed log + advertise/backfill shape;
  validates the linear per-author catch-up. [borrow]
- `prior-art/pears/` (Hypercore) — signed append-only log, sparse replication from
  a head; validates "advertise head, fetch the gap." [validates]
- `prior-art/iroh/` — gossip vs direct-stream cost model; informs §3.1 (gossip is
  right-sized for tiny/rare payloads). [validates]
- Internal: B-1 `HeadsSummary` + B-4.4 direct-stream backfill are the in-tree
  reference design B-12 deliberately scales down.

Runner-up paradigm rejected: direct-stream request/response (B-4.4 rails) — see §3.1.

## §10 Out-of-scope future work — explicit deferrals

- ~~Direct-stream distribution backfill (promote from gossip if scale demands).~~
  **DONE — landed in §14** (promoted not for scale but because gossip-push could
  not catch up a late joiner — see §13).
- Signed summaries.
- Durable archive + GC across restart → **B-9**.
- Install-time staleness gate (consume the §3.7 surface) → install-flow follow-on.
- Publication catch-up for peers >1024 releases behind.

## §11 Estimate

~2–3 focused days. 7 TDD tasks; the heaviest is the async receive-path refactor
(T3) and the iroh-tier test settle-timing (T6). Risk-adjusted: 4 days if the
summary-payload crate placement (§12 Q1) forces a dependency-inversion shuffle
like B-11's network↔distribution fix.

## §12 Open questions for the plan writer

1. **Where do `RevocationHeads`/`PublicationHeads` live?** `crates/distribution`
   (re-exported through `crates/network` as the event envelopes are) is the natural
   home, but confirm it does not reintroduce a cycle — B-11 inverted the
   `network ↔ distribution` edge (distribution no longer depends on network).
   Putting the summary types in `crates/distribution` and having `crates/network`
   depend on them (it already does for `RevocationEvent`) is consistent; verify.
2. **Should the on-start broadcast wait for topic-join settle** before publishing
   the first summary on iroh, or is fire-and-forget + the periodic tick enough?
   Decide empirically in T6 (the event-DAG publishes its first `HeadsSummary`
   eagerly with no settle wait — likely fine).
3. **Bucket parameters** for `distribution_push_limit` (capacity / refill). Start
   from `DriftRateLimit`'s defaults; tune if the flood test needs it.
4. **`distribution_sync_tick` default** — 30 s proposed; confirm against the iroh
   test's settle window so the test isn't waiting on a slow tick (the on-start
   broadcast should make the test independent of the tick).
5. **`stale_authors` clock injection** — take `now` as a parameter (testable) vs.
   read `SystemTime::now()` internally. Proposed: parameter, for deterministic
   tests.

## §13 Implementation finding — gossip-push fails for late joiners over real iroh (§3.1 must be revised)

**Status: the §3.1 transport decision is wrong for the stated goal. Discovered
during T6 (the iroh-tier test), 2026-05-29.**

The gossip re-broadcast / push-on-behind design (§3.1, §3.4) works at the
MemNetwork tier (5 kernel-tier tests green) but **does not catch up a late joiner
over real iroh-gossip**. Instrumented run (one late joiner B, one ahead peer C,
15s window):

- B broadcast its empty head **160×** → C received **0**.
- C broadcast its head **155×** → B received **1** (and B, being the *behind*
  peer, takes no action — it waits for a push that never comes because C never
  heard B's summary).

Root cause — two compounding facts about iroh-gossip (HyParView + Plumtree):

1. **Delivery is asymmetric for a fresh joiner.** Eager-push flows
   established→joiner; the joiner→established path is lazy (IHAVE/GRAFT) and does
   not deliver within the test window. So the *behind* peer's summary never
   reaches the *ahead* peer — and "ahead pushes on hearing a behind summary"
   (§3.4) never triggers.
2. **Identical messages are content-deduplicated.** Periodic re-broadcast of the
   same summary cannot recover, because after the first propagation the rest are
   suppressed by message-id.

This is the **same reason the event DAG uses pull** (`HeadsSummary` advertisement
+ `request_heads` direct-stream backfill, B-1/B-4.4), **not re-broadcast**, for
late joiners: a late joiner fundamentally cannot receive already-propagated
historical gossip; it must *pull*. §3.1 reasoned "distribution events are tiny,
so gossip is adequate" and missed that size is irrelevant — the late-joiner
delivery problem is structural, not bandwidth.

**Corrected design direction (supersedes §3.1 / §3.4):** mirror the DAG.
- The **ahead** peer's summary broadcast reaches late joiners (established→joiner
  works — proven: 1 delivery got through). Keep `RevocationHeads`/`PublicationHeads`
  as the *advertisement*, but have them carry the advertiser's `PeerPubkey`
  (`signed_by_peer`, as `HeadsSummary` does).
- The **behind** peer, on hearing a summary with `remote > local`, **pulls** the
  missing envelopes by **direct-dialing** the advertiser (QUIC direct-stream,
  exactly as `request_heads` dials a peer — this bypasses the Plumtree asymmetry
  because it is a point-to-point connection, not gossip). This requires a
  distribution-flavored direct-stream request/response (a new `Network` method or
  a generalization of the existing direct-stream protocol to carry
  `Revocation`/`Publication` envelopes) + a handler that serves from the kernel
  archive. This is an **ABI/transport change** — exactly the runner-up §3.1
  rejected.

**Landed state (updated 2026-05-29 after §14 implemented):** the corrected
direct-stream pull transport (§14) is **implemented and green**. The kernel
archive, advertiser-bearing summary variants, on-start/periodic broadcast, and
staleness surface remain; the push-on-behind handlers and the amplification
rate-limit are **removed** and replaced by pull (`request_distribution` +
`install_distribution_handler` + `DISTRIBUTION_REQUEST_ALPN`) gated by a
per-advertiser dial-limit. The MemNetwork-tier acceptance suite
(`stale_backfill.rs`: range catch-up, latest-wins publication, mismatched-author
guard, dial-limit, staleness) is green, and the two iroh-tier tests in
`iroh_stale_backfill.rs` now pass over real iroh-gossip (no longer `#[ignore]`d).
**B-12 closes `distribution.md` §10.7** via the corrected transport.

## §14 Corrected transport design — direct-stream pull (supersedes §3.1/§3.4)

*Designed 2026-05-29 after the §13 finding; user chose "design the pull transport
first" before implementing.*

### §14.1 Mechanism

Mirror the event DAG's late-joiner path (advertise → pull), the one shape proven
to work over real iroh-gossip:

1. **Advertise (unchanged).** Each peer broadcasts `RevocationHeads` /
   `PublicationHeads` on the per-author distribution topics (on-start + periodic
   tick). The advertisement reaches late joiners reliably (established→joiner
   eager-push — proven in §13). Summaries now carry `advertiser: PeerPubkey`
   (already landed).
2. **Detect + pull (new).** A peer that hears a summary with `remote_seq > local`
   (it is *behind*) **dials the advertiser** and issues a direct-stream
   distribution-backfill request for the missing range. The dial is a
   point-to-point QUIC connection, bypassing the Plumtree joiner→established
   asymmetry that defeated gossip-push.
3. **Serve (new).** The advertiser's kernel handler reads its signed-envelope
   archive (`revocation_archive` / `publication_latest`, already landed) and
   streams the missing envelopes back.
4. **Apply (reuse).** The behind peer feeds each received envelope through the
   existing `handle_revocation` / `handle_publication` path (verify-edge → apply →
   surface), idempotent via the monotonic-seq check.

The push-on-behind handlers (`handle_revocation_heads` push branch) and the
amplification rate-limit are **removed** — pull replaces push, so a forged
summary no longer weaponises an ahead-peer (a behind peer only ever pulls *for
itself*; a forged-high summary just makes the hearer waste one dial, bounded by a
per-advertiser dial rate-limit). The summary `last_observed_seq` + `advertiser` +
the archive + the staleness surface all stay.

### §14.2 Transport fork — NEW dedicated method (picked) vs. generalize `request_heads`

**Decision: a new, parallel direct-stream protocol for the distribution ledger**
(`Network::request_distribution` + `install_distribution_handler` + a new ALPN),
structurally mirroring the B-4.4 heads-request protocol.

**Runner-up: generalize the existing `request_heads`** to carry a union
`{ Event | Revocation | Publication }` envelope on one stream/ALPN. *Rejected* —
it mutates the proven, wire-frozen event-DAG backfill: `HeadsStream`'s item type,
`HeadsResponder::send`, the response wire frames (`canonical_bincode(Event)` →
enum-tagged), and the DAG serve/drain paths would all churn for zero DAG benefit,
coupling two ledgers with genuinely different request schemas (DAG hash-chains
with `prev`/`deps` vs. linear distribution seq-ranges). The new-method cost is
~2 extra files of *mechanical* scaffolding that copy a proven template; the
generalize cost is regression risk in load-bearing, frozen code. Additive ABI
(out-of-tree `Network` impls add a method) beats mutating an existing surface.

### §14.3 New wire + trait surface (crates/network + crates/types)

```rust
// crates/types (beside the existing DirectHeadsRequest) or crates/distribution
pub enum DistributionLogKind { Revocation, Publication }       // which ledger
pub struct DistributionBackfillRequest {
    pub author: AuthorPubkey,          // whose log
    pub kind: DistributionLogKind,
    pub from_seq: u64,                 // send envelopes with seq > from_seq
}
// response item — narrow (cannot carry an Event by construction)
pub enum DistributionEnvelope {
    Revocation(RevocationEvent),
    Publication(PublicationEvent),
}

// crates/network
pub const DISTRIBUTION_REQUEST_ALPN: &[u8] = b"myrhiza/distribution-request/1";
pub struct DistributionStream { /* mpsc::Receiver<Result<DistributionEnvelope, _>> */ }
pub struct DistributionResponder { /* mpsc::Sender<...> */ }
pub trait DistributionHandler: Send + Sync + 'static {
    async fn handle(&self, requester: PeerPubkey,
                    request: DistributionBackfillRequest,
                    responder: DistributionResponder);
}
pub type ArcDistributionHandler = Arc<dyn DistributionHandler>;

// Network trait — two additive methods
async fn request_distribution(&self, peer: PeerPubkey, req: DistributionBackfillRequest)
    -> Result<DistributionStream, NetError>;
fn install_distribution_handler(&self, handler: ArcDistributionHandler);
```

Framing identical to heads-request: length-prefixed (u32-BE) canonical-bincode
over a QUIC bi-stream; MemNetwork double invokes the handler in-process (no
framing), mirroring `memory.rs`.

### §14.4 Kernel wiring (crates/kernel/src/runtime.rs)

- `KernelDistributionHandler { tx, installed_authors }` installed alongside the
  existing `KernelRequestHandler`; validates the request's `author` is one it
  serves, forwards into a new `distribution_req_rx` mailbox (8th select arm).
- `serve_distribution_request`: reads `revocation_archive[author]` (range
  `from_seq+1..=max`) or `publication_latest[author]` (single, if its seq >
  from_seq); streams `DistributionEnvelope`s. Snapshot before `await` (the
  borrow-discipline `serve_direct_heads_request` uses).
- `handle_revocation_heads` / `handle_publication_heads`: replace the *push*
  branch with — on `remote < local` is now irrelevant; on `remote > local`
  (we are behind) and a per-advertiser dial-limit admits, call
  `issue_distribution_backfill(advertiser, author, kind, local_seq)`.
- `issue_distribution_backfill` + `drain_distribution_response`: mirror
  `issue_direct_backfill` / `drain_heads_response`, feeding pulled envelopes into
  the existing apply path (e.g. via `distribution_rx` as
  `GossipMessage::Revocation/Publication`, reusing `handle_distribution_message`).

### §14.5 Tests

- MemNetwork: keep the existing `stale_backfill.rs` acceptance assertions
  (late-joiner catch-up, range, latest-wins, staleness) — rewire the mechanism
  from push to pull underneath; assertions on the *observable outcome* are
  unchanged. Replace the amplification-flood test with a dial-rate-limit test.
- iroh: un-`#[ignore]` the two `iroh_stale_backfill.rs` tests — they must now pass
  over real iroh-gossip (this is the §10.7 closure criterion). Add a
  `request_distribution` wire round-trip test in `crates/network` mirroring the
  heads-request transport test.
- wire-freeze: pin the new request/envelope encodings if the suite covers the
  direct-stream wire (the heads-request types' precedent).

### §14.6 Touch-list

`crates/types` (or `crates/distribution`): request/kind/envelope types.
`crates/network/src/{lib,request,iroh_transport,memory}.rs`: ALPN, stream,
responder, handler trait, two `Network` methods + both impls (parallel to
heads-request). `crates/kernel/src/runtime.rs`: handler, serve, issue, drain, 8th
select arm, swap push→pull in the heads handlers. `crates/test-utils`: install the
distribution handler in both harness `spawn_peer`s (as they install the heads
handler). Estimate ~1.5–2 focused days.
