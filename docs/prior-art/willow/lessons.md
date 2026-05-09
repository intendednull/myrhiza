**Date:** 2026-05-09
**Status:** active
**Subject:** Willow — validates / avoid / borrow synthesis

The load-bearing decision file. Each entry is an explicit recommendation
to Myrhiza spec authors: lift this with confidence (validates), explicitly
reject this (avoid), or copy this pattern (borrow). Provenance — Willow
spec or code path — is named so the claim can be re-examined.

See also: [README.md](README.md), [runtime-vision.md](runtime-vision.md),
[open-problems.md](open-problems.md), [state-machine.md](state-machine.md),
[testing.md](testing.md).

## Validates — Willow shipped + works + Myrhiza inherits with confidence

These are choices Willow made, paid for, and is glad of. Myrhiza adopts
them by default unless a specific Myrhiza-context reason emerges.

### Per-author Merkle DAG with structural equivocation prevention

Willow's `EventDag::insert` enforces `seq == latest_seq + 1` and
`prev == current_head` *before* signature verification (see
[state-machine.md](state-machine.md)). The combination makes per-author
equivocation **structurally impossible** — an author cannot fork their
own chain without producing the same `(seq, prev)` twice, which the DAG
rejects at insert time. This is cleaner than detection-after-the-fact
and is the right Myrhiza default.

Provenance: `crates/state/src/dag.rs:130-230`,
`docs/specs/2026-04-01-per-author-merkle-dag-state-design.md`.

### iroh as transport

Two years of running Willow on iroh has surfaced no architectural
regrets. Iroh handles native + WASM in one transport story (no
`#[cfg]`-gated network paths in lib crates), and the gossip + blob-fetch
+ ALPN routing primitives are exactly the right shape for a P2P kernel.
Provenance: PR #636 §"What stays the same"; `crates/network/src/iroh.rs`
ships in production.

### Ed25519 identity

Author identity rooted in Ed25519 signatures over the event envelope.
Signature verification on every received event. ~50 µs cost is dominated
by anti-DoS structural caps (verified before signature check). PR #636
keeps this; Myrhiza inherits.

### `Network` trait + `MemNetwork` test double

The `Network` / `TopicHandle` / `BlobStore` trait family in
`crates/network/src/traits.rs`, with `IrohNetwork` for production and
`MemNetwork` for tests, is a cornerstone of Willow's test architecture.
It lets `crates/client/src/tests/multi_peer_sync.rs` exercise multi-peer
gossip without QUIC, relays, or wall-time delays. Myrhiza needs an
equivalent in-memory transport double **early** (before the test pyramid
calcifies on real-iroh-only).

Provenance: `crates/network/src/{traits,iroh,mem}.rs`; CLAUDE.md
"Testing Strategy".

### Actor-only state discipline

`docs/specs/2026-04-26-state-management-model-design.md` codified what
Willow learned the hard way: lock-based shared state in lib crates
drifted into multi-lock atomicity bugs (`SearchIndexHandle` with four
independent `parking_lot::Mutex`es). The rule "shared mutable state in
lib crates lives inside an actor" plus the documented decision tree
(StateActor / iroh-callback-lock / OnceLock / AtomicBool /
`Rc<RefCell>` / Leptos signal / watch-channel) is the right default for
Myrhiza.

The `// state: lock-ok — <reason>` comment convention for legitimate
exceptions is also worth lifting verbatim.

### Pre-check equals apply (the absent-divergence invariant)

Worth stating explicitly: **the strong invariant is "pre-check and
authority verdict cannot diverge."** PR #636's mechanism (same WASM
function, dry-run mode) is one way to enforce it; in shipped Willow
today the invariant holds because both go through
`willow-state::materialize::apply_event` in trusted in-process Rust.
The mechanism changes; the invariant is non-negotiable.

Myrhiza's CLAUDE.md already calls this out; this entry just names the
underlying truth: **drift between pre-check and apply is a correctness
bug**. Whatever runtime mechanism enforces non-drift, it is enforced.

### `HeadsSummary` sync protocol

`HeadsSummary { heads: BTreeMap<EndpointId, AuthorHead> }` is the compact
peer-state advertisement Willow uses for sync. Adding a peer's
contribution to a topic costs O(authors) bytes, not O(events). The
`compare_chains` four-state response (`Ahead` / `Behind` / `Synced` /
`Forked`) is the right level of abstraction. Myrhiza inherits the shape
even if the wire encoding evolves.

Provenance: `crates/state/src/sync.rs:21-201`.

### Dual-target lib-crate compilation

Every Willow lib crate compiles to both native and `wasm32-unknown-
unknown`. Discipline: no `std::fs`, no `std::time::SystemTime`, no
`std::thread`, no tokio in lib crates; gate platform paths with
`#[cfg(target_arch = "wasm32")]`. This survived two years and lets
Willow share ~80% of code between native workers and the browser
client. **Myrhiza's kernel layer must keep this discipline**;
application WASM components are built once and loaded by whichever
kernel hosts them.

### Bincode-with-sorted-collections (postcard envisioned for state-digest)

Today's Willow encodes via **bincode** over `BTreeMap`/`BTreeSet`
(sorted-keyed) — see `crates/state/src/event.rs:532`,
`crates/state/src/sync.rs:100`, `crates/state/src/types.rs:342`.
The sorted-collection discipline is the load-bearing piece (deterministic
iteration order); the format choice is bincode in shipped code. PR #636
envisions migrating the canonical `state-digest()` form to **postcard**
with sorted collections (PR #636 §"Determinism, in detail" calls postcard
"the existing-codebase precedent" — that claim is forward-looking, not
current). Myrhiza inherits the sorted-collection discipline; format
selection (postcard vs bincode vs other) is a determinism-enforcement
child-spec decision.

### bech32m-with-HRP identifiers

`docs/specs/2026-04-24-bech32-identifiers.md` establishes a typed
identifier scheme with HRP prefixes. The shipped HRPs are: `wpeer`
(peer pubkey), `wserver` (server ID), `wevent` (event hash), `wchan`
(channel), `winv` (invite), `wrelay` (relay), `wblob` (blob hash). No
secret material ever uses bech32m (no `wsecret`-style HRP). HRPs prevent
identifier confusion at API boundaries; the checksum catches transcription
errors. Generic enough to lift directly into Myrhiza (re-derive HRPs for
the Myrhiza namespace, keep the scheme). See [`identity.md`](identity.md).

### Two-policy `PendingBuffer` eviction

Out-of-order events are buffered with **age-based** eviction
(`DEFAULT_PENDING_MAX_AGE_MS = 1h`) **and** **capacity-based** eviction
(`DEFAULT_PENDING_MAX_ENTRIES = 10_000`, with per-author sub-cap of
`max_entries / 50` to thwart Sybil-shaped flooding). The two policies
are independent so neither alone gates the other. Myrhiza needs the
same shape; defaults will re-tune for non-chat workloads.

Provenance: `crates/state/src/sync.rs:178-201`; SEC-V-08 sub-cap is the
specifically-tuned-for-Sybil bit.

### Genesis-event-defines-server-id

The first event of a server must be `EventKind::CreateServer`; that
event's hash becomes the `server_id`. Topic-IDs derive from this. This
is **content-addressed identity for app instances** — clean, no
allocator, no registry. Generic enough that Myrhiza inherits the
pattern as "first state event defines instance identity." The variant
name (`CreateServer`) is chat-specific and stays in the app.

## Avoid — Willow shipped a thing PR #636 explicitly rejects, or that turned out wart-shaped

These are the choices PR #636 names as needing change, plus a couple of
shipped patterns that Myrhiza should NOT inherit even though they
"work" in Willow today.

### `EventKind` enum baked into the kernel-equivalent

Today's `willow-state` is "kernel-equivalent" (the rest of the system
treats it as authoritative) but is **not generic over payload**.
`EventDag` is concrete over `EventKind`, which has 22 chat-specific
variants (`Message`, `CreateChannel`, `RotateChannelKey`, etc.). PR
#636 explicitly splits this: a payload-agnostic kernel half retains
`Event`, `EventDag<P>`, sync; the chat-specific half becomes the
`chat-server` app's `state-apply` component.

**Myrhiza must NOT bake any payload-shape into the kernel.** This is
the single biggest "if Myrhiza just lifts willow-state as-is, it has
already failed" item.

Provenance: `crates/state/src/event.rs:280-468`; PR #636 §"What changes
about Willow" (`willow-state` splits).

### Centralized `required_permission()` table

`willow-state::materialize::required_permission()` is a giant `match`
mapping `EventKind` variants to a kernel-known `Permission` enum
(`SyncProvider`, `ManageChannels`, `ManageRoles`, `SendMessages`,
`CreateInvite`). PR #636 makes this **per-app**: each app defines its
own permission set and exports its own authority predicate.

Avoid Myrhiza making the same shape: do not create a kernel-known
permission enum, even one that "could be reused." Each app owns its
permissions; the kernel only sees opaque "did the app's authority
predicate accept this event?" verdicts.

Provenance: `crates/state/src/materialize.rs:297-346`.

### `Permission` enum hardcoded in `willow-state`

Same shape as above, viewed from the schema side. Even though
`Permission` was carefully designed for chat, lifting it into Myrhiza
(under any name) would be a form of premature kernel-ization of a
domain concept. Apps define permissions.

### Owner-override carve-out for governance

`materialize::apply_event` has a special path
(`materialize.rs:213-218`) where the genesis author can push governance
through unilaterally without the threshold-vote machinery. This is
**one valid app-level authority pattern**, not a kernel built-in. PR
#636's framing — kernel only sees "did the app's authority predicate
accept?" — makes this an app-internal decision. Apps that want owner
override implement it; Myrhiza must not make it a kernel concept.

Provenance: `crates/state/src/materialize.rs:213-218`.

### Single global `EventDag` per server

Today's `EventDag` is per-server. Multi-topic peers (workers
materializing many servers) hold many `EventDag`s, but each is its own
process actor. The runtime model PR #636 commits to is **per-topic
state-materialization actors owning one state component instance each**.
Myrhiza should design for the multi-topic-on-one-peer case from the
start.

### Single-vendor rust-monolith approach for the application surface

Willow today ships chat as Rust code linked into the same binary as the
kernel. Worker code, client code, and chat semantics share types
through `willow-common`. This is an app-shape Willow accepted while
chat was the only product. **Myrhiza must reject this from day one**:
the kernel must not link app types, must not import app crates, must
not share serde formats with apps. Apps are sandboxed WASM.

PR #636 does not "reject" this — there's no way to reject it for a
shipping monolith — but the runtime work is precisely the inversion of
this shape.

### `timestamp_hint_ms` with split semantics (signed; not for ordering; materialized into derived state)

Willow signs `timestamp_hint_ms` into the event but does **not** use it
for DAG ordering or merge tie-break (`event.rs:496-498`); ordering is
content-causal-plus-lex-hash. The field IS consumed on the
materialization side — `Channel.last_activity_hlc`, ephemeral-channel
idle thresholds (`materialize.rs:521`, `ephemeral.rs`). PR #636 talks
about HLC as a deterministic helper for `apply` — implying timestamps
may become more load-bearing in the runtime model.

The current shipped state is split-semantics-shaped: a signed field
that's authoritative for some derived state but irrelevant for ordering.
That split is a review-trap. Myrhiza should pick one model and document
it: either the time field is ordering-load-bearing (and apps must reason
about HLC monotonicity in `state-apply`), or it is materialization-only
and signing it is documented as advisory. Don't leave the semantics
implicit.

### Lockstep / centralized-state distinctions

Willow has no client/server distinction in the protocol. Validates as a
choice — Myrhiza inherits "no client/server distinction in the kernel"
as a hard rule. This is an "avoid the temptation," not "avoid a thing
Willow did." Listed here because future Myrhiza contributors may be
tempted to re-introduce the distinction for "performance" or "trust";
the answer is no.

## Borrow — patterns to lift mechanically

Workflow + tooling + organization patterns Willow ships that Myrhiza
should copy directly.

### Test-tier hierarchy: state > client > browser > Playwright

`docs/specs/2026-04-21-e2e-test-architecture-design.md` is the
load-bearing test-architecture spec. Decision tree mandates the lowest
tier that can cover the behaviour. The full spec, the decision tree,
and the **rewrite trigger** ("Playwright test fails because selector
drifts, not behaviour broke — migrate it down on the same commit")
should all be lifted into Myrhiza. State-tier-tests-on-state-apply-WASM
is the natural Myrhiza analogue. See [testing.md](testing.md).

### Event-based-waits + `WillowTestHooks` pattern

`docs/specs/2026-04-27-event-based-waits-design.md`. Replaces
`waitForTimeout(ms)` sleeps with three deterministic primitives: push
events (`__willowEvent` binding), pull state (`expect.poll`), and fake
clocks (`page.clock`). Forbids `waitForTimeout` via ESLint rule;
documents anti-patterns. **Myrhiza inherits this whole** before its
integration-test suite paints itself into the same flake corner Willow
spent weeks paying off.

### Spec-then-plan discipline with date-prefix naming

`docs/specs/YYYY-MM-DD-<name>-design.md` for "what we're building
toward"; `docs/plans/YYYY-MM-DD-<name>.md` for migration steps.
Already lifted into Myrhiza CLAUDE.md verbatim. Worth naming as a
deliberate borrow because it is unusually disciplined for a Rust
codebase.

### CLAUDE.md "no shortcuts / root-cause every bug / failing test is a question"

The dev-guidelines section of Willow's CLAUDE.md was pasted into
Myrhiza's CLAUDE.md essentially unchanged. Worth flagging here so future
auditors know the lineage and don't accidentally diverge the two
without reason.

### Grove-style multi-instance hosting

Willow's "Grove" concept — one peer hosts multiple servers (state
instances) — translates directly to Myrhiza: one peer hosts multiple
app instances of multiple apps. The actor topology Willow uses (one
actor per server, mailbox-serialized state mutation, snapshot custody)
is the natural Myrhiza shape. Borrow the topology; rename the concept.

Provenance: `crates/client/`, `crates/worker/`,
`docs/specs/2026-04-12-willow-channel-removal.md`.

### Staged-rollout PR plans (UI Phase 0 → 3c)

`docs/plans/2026-04-19-ui-phase-0-foundation.md` through
`2026-05-08-ui-phase-3c-reactions-pins.md` — a 12-PR sequence breaking
a UI rebuild into individually-shippable, individually-reviewable
chunks. Myrhiza's kernel work is at least as large; the staged-PR
discipline is worth lifting verbatim.

### Skill set + `.claude/skills/` vendored layout

Brainstorming, writing-plans, executing-plans, TDD, code-review,
worktrees, parallel-agents, simplification skills. Already lifted into
Myrhiza. Worth naming so the inheritance is explicit.

## Open questions Willow surfaces that Myrhiza inherits

Each of these is a real design decision Willow named without resolving;
Myrhiza will face them whether or not it acknowledges them up front.

- **Distributed maintenance + Sybil-resistant participation
  enforcement** — see `research-notes-distributed-maintenance.md`. The
  existing permission/invite trust graph is a Sybil-relevant advantage.
- **Multi-device identity** — long-term identity, short-lived per-device
  signing key. PR #636 calls out "structurally same as behaviour
  identity"; both want one mechanism.
- **MLS adoption** (deferred to a future MLS-over-Willow spec; will
  re-use Myrhiza's `prior-art/mls/` corpus).
- **Cross-app authority composition** — out of scope for v1; v2 hook
  shape unspecified.
- **Hot-reload** — deferred to v2 in PR #636; component update is
  restart for v1.
- **Snapshot portability across component-version upgrades** —
  migration story when an app's state component is updated.
- **Topic-ID rotation** through dumb relays without leaking next-topic
  IDs (the existing epoch-rotation work needs to land in this new
  shape).
- **Resource-limit defaults** for per-instance fuel + memory budgets.
- **Handle namespace ownership** when two apps install keys under the
  same opaque handle.
- **Behaviour coordination primitives** — leader election, dedup; PR
  #636 leaves to apps. Should Myrhiza offer a kernel primitive?

See [open-problems.md](open-problems.md) for canonical sources to
consult when designing each.

## Repo

- GitHub: [github.com/intendednull/willow](https://github.com/intendednull/willow)
- PR #636 (master runtime spec, draft): [intendednull/willow#636](https://github.com/intendednull/willow/pull/636)

## Sources

- `/tmp/willow-pr-636.diff` (843 lines, full PR diff).
- `/mnt/storage/projects/willow/CLAUDE.md`.
- `/mnt/storage/projects/willow/docs/specs/2026-04-01-per-author-merkle-dag-state-design.md`.
- `/mnt/storage/projects/willow/docs/specs/2026-04-12-state-authority-and-mutations.md`.
- `/mnt/storage/projects/willow/docs/specs/2026-04-21-e2e-test-architecture-design.md`.
- `/mnt/storage/projects/willow/docs/specs/2026-04-26-state-management-model-design.md`.
- `/mnt/storage/projects/willow/docs/specs/2026-04-27-event-based-waits-design.md`.
- `/mnt/storage/projects/myrhiza/CLAUDE.md` (cross-check for what is
  already lifted).
- Code: `crates/state/src/{event,dag,materialize,sync}.rs`,
  `crates/network/src/{traits,iroh,mem}.rs`,
  `crates/client/src/tests/multi_peer_sync.rs`.
