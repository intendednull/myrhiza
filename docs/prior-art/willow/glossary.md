**Date:** 2026-05-09
**Status:** active
**Subject:** Willow — glossary of state, sync, authority, and runtime terms

Willow-specific and Willow/Myrhiza-overlap terms. Cited to source spec
or file path when non-obvious. Shipped terms are unmarked; PR #636
aspirational terms are flagged "(PR #636)".

See also: [state-machine.md](state-machine.md),
[authority.md](authority.md), [determinism.md](determinism.md),
[README.md](README.md).

## State, DAG, and events

**`Event`** — Atomic unit of state mutation. Content-addressed
(`hash` field is the SHA-256 of the signable content), author-signed
(Ed25519). Carries `author`, `seq`, `prev`, `deps`, `kind`, `sig`,
`timestamp_hint_ms`. `crates/state/src/event.rs:478-498`.

**`EventHash`** — SHA-256 of an event's signable content. The event's
identity. Implements `Ord` as lex byte comparison, used for
deterministic topo-sort tiebreaking. `EventHash::ZERO` = the all-zero
hash, used as `prev` for an author's first event.
`crates/state/src/hash.rs`.

**`EventKind`** — The 22-variant enum of state mutations: governance
(`Propose`, `Vote`), permissions (`GrantPermission`,
`RevokePermission`), structure (`CreateChannel`, `CreateRole`, …),
chat (`Message`, `FileMessage`, `EditMessage`, `DeleteMessage`,
`Reaction`), identity (`SetProfile`, `UpdateProfile`), encryption
(`RotateChannelKey`), pinning, server metadata, per-identity mute.
Chat-specific today; under PR #636 this becomes app-defined.
`event.rs:280-468`.

**`EventDag`** — In-memory store of all known events for one server,
indexed by `EventHash` plus per-author chain index plus per-author head
map. Source of truth from which `ServerState` is derived.
`crates/state/src/dag.rs:103-112`.

**`prev`** — Hash of the author's previous event in their own chain.
`EventHash::ZERO` for the first event. Per-author chain integrity is
enforced at insert (`prev == current_head`).

**`deps`** — Cross-author causal heads "this event has seen." Advisory
(soft-accept on unknown deps), capped at `MAX_EVENT_DEPS = 64` for DoS
resistance. Forms the cross-author DAG edges. `event.rs:22, 489-491`.

**`genesis`** — The first event in a server's DAG. Must be
`EventKind::CreateServer`. The server's `server_id` is the hex of
this event's hash; the genesis author becomes the sole initial admin
and the permanent root of trust.
`per-author-merkle-dag-state-design.md` §"Server Identity".

**`ServerState`** — The materialized projection of `EventDag`.
`BTreeMap`/`BTreeSet` everywhere serialized. Holds `channels`, `roles`,
`members`, `messages`, `peer_permissions`, `profiles`, `admins`,
`vote_threshold`, `pending_proposals`, etc. `server.rs:33-104`.

**`materialize`** — The pure function `EventDag → ServerState`.
Topo-sorts the DAG and replays through `apply_event`. Only public
mutation entry alongside `apply_incremental`. `materialize.rs:64-80`.

**`apply_event`** — Internal per-event mutator. Pre-checks permission,
handles governance specially, delegates to `apply_mutation`. Pure.
`materialize.rs:161-202`.

**`apply_incremental`** — Public incremental mutator. Idempotent via
`state.applied_events` dedup. The single public mutation path used by
client / worker / agent. `materialize.rs:92-102`.

**`HeadsSummary`** — Compact representation of "what I know" for
sync: `BTreeMap<EndpointId, AuthorHead>`. Each `AuthorHead` is
`{ seq, hash }`. `sync.rs:21-33`.

**`PendingBuffer`** — Buffer for events whose `prev` references an
unknown event (per-author chain gap = hard gap). Two eviction policies:
age (`DEFAULT_PENDING_MAX_AGE_MS = 1h`) and capacity
(`DEFAULT_PENDING_MAX_ENTRIES = 10_000`, with per-author sub-cap
`max_entries / 50` for SEC-V-08 Sybil resistance).
`sync.rs:178-201`.

**`Snapshot`** — Frozen checkpoint of `(ServerState, HeadsSummary)`
for far-behind-peer bootstrap. Hash is computed from a sorted-heads
canonical encoding. `sync.rs:61-105`.

**HLC** — Hybrid Logical Clock. Per-peer monotone timestamp combining
wall-clock millis with a logical counter. `crates/messaging/src/hlc.rs`.
*Used by the messaging crate for message ordering hints; not used by the
state crate for DAG ordering.* In `willow-state`, `event.timestamp_hint_ms`
is signed and **materialized into derived state** (e.g.
`Channel.last_activity_hlc`, ephemeral-channel idle thresholds —
`materialize.rs:521`, `ephemeral.rs`) but is **not** used for DAG topo-sort
or merge — that is content-causal-plus-lex-hash.

## Authority and governance

**`required_permission`** — The `EventKind → Option<Permission>`
table. Drives the permission-gated tier in `check_permission`. The
catch-all `_ => None` arm is annotated with every variant that returns
`None` so reviewers notice if a new variant is missing (bug #109 lesson).
`materialize.rs:297-346`.

**`Permission`** — Non-admin permissions: `SyncProvider`,
`ManageChannels`, `ManageRoles`, `SendMessages`, `CreateInvite`. Plus
`__UnknownLegacy` sentinel for deserialization back-compat. Note: does
NOT include admin status — that lives in `ServerState.admins`.
`event.rs:50-95`.

**`SyncProvider`** — Permission marking a peer trusted to serve
history. Required for relays. Granted by direct admin
`GrantPermission` event; not implied by membership.

**Owner** — Synonym for *genesis author*. Permanent server creator.
Has unilateral governance override (`materialize.rs:213-218`) and is
protected from removal by the 0-admin guard.

**Admin** — Member of `ServerState.admins`. Implicitly holds every
`Permission`. Granted/revoked only via `ProposedAction::GrantAdmin` /
`RevokeAdmin` through the vote path — structurally separate from the
`Permission` enum to make `GrantPermission`-based escalation
impossible. `event.rs:50-52`, `server.rs:73`.

**`ProposedAction`** — The four governance-vote actions:
`GrantAdmin`, `RevokeAdmin`, `KickMember`, `SetVoteThreshold`. These
cannot be triggered any other way — direct execution is structurally
impossible. `event.rs:217-227`.

**`VoteThreshold`** — `Majority` (default), `Unanimous`, `Count(n)`.
`event.rs:230-239`.

**`GrantPermission`** — `EventKind` variant for direct (no-vote)
permission grants by an admin. Cannot grant admin status (no admin
variant in `Permission`). `event.rs:294-298`.

## Identity and crypto

**`EndpointId`** — A peer's public key. The 32-byte Ed25519 public key
serving as both transport address (iroh endpoint) and authorial
identity. `willow_identity::EndpointId`.

**`Identity`** — Local secret keypair (Ed25519). Lives only in the
kernel-owning process; never leaves the machine. `willow-identity`
crate.

**`Topic`** — iroh-gossip topic ID for a server. Today derived per
server; under the epoch-rotation spec, rotates over time for
unlinkability. PR #636 §"Constraints we accept" defers post-runtime
topic-ID rotation to a child spec.

**`EpochKey`** — A channel key valid for one epoch. Rotated via
`EventKind::RotateChannelKey`, which carries one
`(EndpointId, encrypted_key_blob)` per recipient. Each blob is capped
at `MAX_ENCRYPTED_KEY_BYTES = 128`. `event.rs:25-35, 431-435`.

**`WireMessage`** — Top-level network frame. Carries `Event` payloads
plus other transport messages. `willow-messaging` crate.

## Runtime profiles (PR #636)

**`state-apply`** (PR #636) — The deterministic component entry point
that materializes an event into state. Bound only to the deterministic
helper set: `host.verify-signature`, `host.verify-payload-mac`,
`host.hash`, `host.install-key`, `host.now-hlc-from-event`, `host.log`.
No clock, random, network, FS, or threads. Spec-deterministic floats
discouraged.

**`state-propose`** (PR #636) — The non-deterministic component entry
point that builds a candidate event payload on the originating peer.
May import `host.hlc`, `host.random`, capability-gated `host.seal`. The
kernel re-checks the proposal via `state-apply` in dry-run mode before
signing.

**`interaction`** (PR #636) — Non-deterministic UI/agent component.
Imports `host.broadcast`, `host.subscribe`, `host.kv`,
`host.user-prompt`, plus the UI app's `ui:*` interfaces. Runs per-peer.

**`behavior`** (PR #636) — Non-deterministic bot/bridge/automation
component. Imports `+ host.http`, `host.timer`, `host.identity` (own
keypair, capability-gated). Identity is per-`(peer, instance)`.

**`maintenance`** (PR #636 research notes) — Anticipated fourth
profile: persisters, snapshot providers, sync providers, replay
buffers. Capacity-hinted. Not committed in the master spec yet —
flagged for the participation/free-rider discussion in
`research-notes-distributed-maintenance.md`.

**pre-check** (PR #636) — Dry-run invocation of `state-apply` against
a hypothetical post-state, used by the kernel before signing a
candidate event. **Mechanically the same WASM function** as `apply` —
not "shared logic by convention." Fails closed.

**apply** (PR #636) — Real (non-dry-run) invocation of `state-apply`
during DAG replay. Same code path as pre-check.

**host imports** (PR #636) — WASM host functions exposed to a
component. The set bound to a component is determined by its profile
and its manifest's declared capabilities. State-`apply`'s set is the
deterministic helper set; other profiles get strictly larger sets.

**kernel** (PR #636) — The privileged in-process Rust runtime that
loads components, brokers calls, owns identity / signing / iroh /
storage / capability-arbitration. Apps reach the host only through
declared imports.

**key handle** (PR #636) — Opaque ID by which a component refers to
key material custodied by the kernel. Components hold handles; the
kernel custodies bytes. Secrets do not enter component memory in raw
form.

**`state-digest`** (PR #636) — App-exported function returning
canonical bytes (bincode with sorted `BTreeMap`/`BTreeSet` is the
shipped-Willow precedent; PR #636 proposes postcard going forward)
the kernel hashes for cross-peer convergence checks. Replaces "hash
WASM linear memory," which would diverge trivially due to allocator
behavior.

**`manifest.toml`** (PR #636) — Per-app declaration of version,
component hashes, capabilities required, interfaces imported and
exported. Read by the kernel at install time to derive the
permission summary.

**bundle** (PR #636) — User-facing distribution unit on iroh-blobs.
Hash-pinned, signed by the author. Contains `manifest.toml`,
component WASM modules (state, interaction, behavior),
`schema.wit`. Lazy-loaded and hash-cached per peer.

**submit-and-poll** (PR #636) — Sync ABI pattern for inherently
async kernel calls (gossip broadcast, blob fetch, HTTP, timer). The
component calls a sync host function returning a `request-token`;
the kernel later re-enters the component via an exported
`on-completion(token, result)` handler. Browser jco does not
support async, so this is the v1 shape.

**`request-token`** (PR #636) — Opaque token returned by a
submit-and-poll host call, replayed by the kernel on completion.

## Repo

- GitHub: [github.com/intendednull/willow](https://github.com/intendednull/willow)

## Sources

- `crates/state/src/event.rs`, `dag.rs`, `materialize.rs`,
  `server.rs`, `sync.rs`, `hash.rs` — shipped term definitions.
- `crates/messaging/src/hlc.rs` — HLC algorithm.
- `docs/specs/2026-04-01-per-author-merkle-dag-state-design.md` —
  state machine design rationale (server identity, governance model,
  EventHash ord).
- `docs/specs/2026-04-12-state-authority-and-mutations.md` — authority
  spec, permission tier table.
- PR #636 `docs/specs/2026-04-27-willow-runtime/README.md` —
  runtime profiles (state-apply / state-propose / interaction /
  behavior), kernel, host imports, key handles, state-digest, bundle,
  manifest, submit-and-poll, request-token.
- PR #636 `docs/specs/2026-04-27-willow-runtime/research-notes-distributed-maintenance.md`
  — maintenance profile (research notes, not yet master-spec).
