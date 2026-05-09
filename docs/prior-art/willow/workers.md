**Date:** 2026-05-09
**Status:** active
**Subject:** Willow — workers (relay, replay, storage), peer-as-worker model, PR #636 generalization

Willow ships three worker binaries: `willow-relay` (transport plumbing),
`willow-replay` (in-memory bounded state-sync), `willow-storage` (archival
SQLite history). All are **regular peers running the same code** with a
different `WorkerRole` impl, gated by `SyncProvider` permission. PR #636
proposes generalizing them into commodity peer hosts that load arbitrary
WASM state components — a substantial trust-model shift not yet implemented.

See also: [actors.md](actors.md), [apps.md](apps.md),
[state-machine.md](state-machine.md), [README.md](README.md).

## `willow-worker` shared library — shipped today

`crates/worker/src/lib.rs` re-exports `WorkerRole`, `WorkerConfig`, and
`runtime::run`. `crates/worker/src/runtime.rs:20-83` is the entry point
every worker binary calls:

```rust
pub async fn run<N: Network>(role: Box<dyn WorkerRole>, config: WorkerConfig, network: N) -> ...
```

It spawns four actors via `willow_actor::System` (worker-nodes spec
§"Concurrency Model" lines 184-242):

1. **Network actor** — owns gossip subscription. Receives gossipsub
   events, dispatches to state actor; receives outbound and publishes.
2. **State actor** — owns the `WorkerRole` impl (the only mutable state).
   `WorkerRole::on_event(&mut self, &Event)` and `handle_request(&mut self,
   WorkerRequest) -> WorkerResponse` are called sequentially from this
   actor's loop. No locks anywhere.
3. **Heartbeat actor** — every 10s queries state actor for `WorkerRoleInfo`,
   broadcasts a `WorkerAnnouncement` on `_willow_workers`.
4. **Sync actor** — every `sync_interval_secs` (default 30) broadcasts a
   `SyncRequest` with current heads, ensuring active convergence rather
   than just passively receiving gossip.

A `tokio::sync::watch::channel(false)` ready signal (`runtime.rs:40`) blocks
the network actor from draining gossip events until state-actor init
finishes. Annotated as a coordination signal, not shared mutable state.

`WorkerRole` (`crates/worker/src/types.rs`):

```rust
pub trait WorkerRole: Send + 'static {
    fn role_info(&self) -> WorkerRoleInfo;
    fn on_event(&mut self, event: &Event);
    fn handle_request(&mut self, req: WorkerRequest) -> WorkerResponse;
}
```

The state actor calls these sequentially — `&mut self` is safe because no
other task holds a reference. The load-bearing shape: a worker is just a
peer with a `WorkerRole` impl plugged into the standard four-actor runtime.

## `willow-relay` — transport plumbing only

`crates/relay/src/lib.rs:1-43` (1150 lines + 239-line `main.rs`) declares
**transport-only**:

- No Ed25519 signature verification on relayed messages.
- No event-sourced state-machine application.
- No permission, role, or governance enforcement.
- No content-based routing or filtering.

Only semantic work is syntactic topic-string validation (`topic_str_is_valid`)
as a DoS guard. TCP/WebSocket bridge for browser peers; iroh relay HTTP at
`localhost:3340` in dev. Trust model (`lib.rs:24-42`):

> The relay is a regular client in the DAG sync protocol. Its Ed25519
> identity carries no implicit authority. A hostile relay can affect
> availability (drop, delay, reorder) but cannot forge events, bypass
> permissions, or corrupt state — those invariants are enforced
> cryptographically and deterministically at each client.

DoS guards: `MAX_CONCURRENT_BOOTSTRAP_CONNECTIONS = 1024` (`lib.rs:60`),
`BOOTSTRAP_IO_TIMEOUT = 5s` (`lib.rs:76`), `MAX_TOPICS = 10_000`
(`lib.rs:91`), `MAX_TOPIC_LEN = 256` (`lib.rs:95`). Relay is now lightweight
enough to run multiple instances for redundancy without coordination —
gossipsub handles propagation across the mesh.

## `willow-replay` — in-memory state-sync, bounded

`crates/replay/src/{main,role}.rs` (1598 lines). One in-memory `EventDag`
per server with `MAX_SERVERS = 1000` LRU eviction (`role.rs:19`); per-author
chain capped at `max_events_per_author` (default 1000, CLI-tunable).
`PendingBuffer` for events arriving before chain predecessors with
age-eviction (`DEFAULT_PENDING_MAX_AGE_MS` = 1 hour) and capacity-eviction.

Responds to `WorkerRequest::Sync { server_id, heads, .. }` with event deltas
computed from `HeadsSummary`, or full `Snapshot` for far-behind peers.
Active sync: replay broadcasts its own `SyncRequest` every 30s — workers
actively pull, not just listen passively.

Native-only (`Cargo.toml`, `role.rs:23-26`); `SystemTime`/filesystem always
available. Replay is not built for WASM.

## `willow-storage` — archival SQLite history

`crates/storage/src/{main,role,store}.rs` (425 + 1274 lines).
SQLite-backed full event archive. Handles `WorkerRequest::History {
server_id, channel, before, limit }` with paginated DAG-aware
`HeadsSummary` cursors, and `WorkerRequest::Sync` for catch-up. Reports
`WorkerRoleInfo::Storage { servers_tracked, total_events_stored,
disk_used_bytes }` to drive client worker discovery. Doesn't materialize
`ServerState` but does broadcast `SyncRequest` to discover events it may
have missed, ensuring archive completeness.

## Permission model — workers are peers with jobs

A worker's Ed25519 identity carries **no implicit authority**. Workers are
authorized via `EventKind::GrantPermission { peer_id,
permission: Permission::SyncProvider }` (`crates/state/src/event.rs:54-56`)
— same mechanism as any other peer. `WorkerAnnouncement` peers with
unknown identities are filtered client-side; clients only treat a worker
as authoritative once the grant is observed in the DAG.

CRDT-shape consequence: workers don't authoritatively **own** anything.
They're caches. The DAG is per-author, signature-rooted, content-addressed
— a worker that disappears costs availability, not correctness. A
malicious worker can drop, delay, or reorder; it cannot forge an event
or rewrite history.

Future worker types pre-anticipated in `RoleType` (`worker-nodes-design.md:174-180`):
File, Stream, Bot, Search, Bridge. Each would be a new binary implementing
`WorkerRole` — discovery protocol, permission model, deployment unchanged.

## For Myrhiza

PR #636 §"What changes about Willow" (diff lines 421-444) commits to:

> **Workers become generic peer hosts** that load state components for any
> topic they are subscribed to.

This is **aspirational** — not implemented. Today `ReplayRole` and
`StorageRole` are trusted in-tree Rust impls of `WorkerRole`, calling
`apply_incremental` on a fixed chat-specific `ServerState`. Under the
runtime, a worker subscribed to N topics may execute N distinct,
third-party-authored, attacker-influenceable WASM `state-apply` components
simultaneously. New responsibilities (PR #636 lines 436-443):

- **DoS resistance** — a malicious or buggy state component must not crash
  the worker process or starve other components.
- **Fuel scheduling** — per-event instruction budgets, deterministic by
  spec so cross-peer convergence doesn't drift on under-fueled peers.
- **Per-instance memory caps** — bounded linear memory per component to
  prevent fork-bomb-style memory exhaustion across topics.
- **Fair-share between topics** — one popular noisy app must not starve
  CPU from a dozen quiet ones.
- **Operator deny-lists** — operators must constrain which apps a worker
  hosts. "Run any WASM" is not tenable for a multi-tenant operator.

PR #636 open child-spec questions:

- **Worker capability advertisement** (lines 648-651). Today's
  `WorkerAnnouncement` declares role + capacity. Should it also declare
  hosted app-component hashes so peers can discover "a worker that
  materializes my chat-server app" without out-of-band config? Parallel
  to `docs/specs/2026-04-24-relay-capability-doc.md`.
- **Per-instance fuel + memory budgets** — defaults? (line 647).
- **Snapshot portability across component-version upgrades** — when an
  app's state component updates, do existing snapshots remain valid?
  (lines 658-661).

### Maintenance components — the fourth profile being teased

PR #636's research notes
(`docs/specs/2026-04-27-willow-runtime/research-notes-distributed-maintenance.md`,
diff lines 681-843) reframe maintenance work as **a fourth class of
components** alongside state / interaction / behavior:

> **Maintenance components** — persister, snapshot provider, sync provider,
> replay buffer. Optional in any app's bundle. Loaded by peers that opt to
> contribute, with kernel-known capacity hints.

Today's `replay` and `storage` workers are exactly the maintenance roles
this profile would cover, baked into worker binaries instead of being WASM
components. Under the new framing, `willow-replay` and `willow-storage`
become a generic peer + a maintenance component the kernel loads on demand.
Scaling becomes emergent: more peers running an app's maintenance
components → more sync/persist capacity, automatically. No separate
work-tracking subsystem.

The **load-bearing open problem**: **participation enforcement under
Sybil**. A custom client that doesn't run maintenance components,
multiplied by spinning up many identities, free-rides on honest
participants — and the honest peers' load grows with the cheaters'
identity count. The notes survey 20+ years of literature (Adar & Huberman
free-riding measurement, BitTorrent's choking algorithm, EigenTrust, BAR
Gossip, SybilGuard / SybilLimit, Holochain's DHT-responsibility model,
IPFS Bitswap) and flag the existing permission/invite trust graph as a
possible Sybil-resistance input — unique to Willow over generic P2P
systems. The problem is **not framed as solved**; it's the next-session
research agenda. The master spec deliberately defers drafting the
maintenance-profile section until that lands.

### What survives, what changes

Survives: workers as peers with no implicit trust + `SyncProvider` gate;
four-actor runtime (network/state/heartbeat/sync) inside the kernel;
`WorkerAnnouncement` discovery protocol (likely extended with hosted
app-component hashes); CRDT-shape (workers as caches).

Changes: `WorkerRole` impls (`ReplayRole`, `StorageRole`) become generic
kernel code that loads WASM state components by hash; operator config
grows app allow/deny-list controls; per-instance fuel and memory budgets
become first-class; maintenance-as-component reframing replaces
"worker types as binaries" — today's `Replay`/`Storage` and future
`File`/`Stream`/`Bot`/`Search`/`Bridge` become app-bundle maintenance
components, not separate binaries.

## Repo

- GitHub: [github.com/intendednull/willow](https://github.com/intendednull/willow)

## Sources

- `crates/worker/src/{lib,runtime,types,identity,config}.rs`,
  `actors/{heartbeat,network,state,sync}.rs`
- `crates/relay/src/lib.rs:1-100` — transport-only declaration, trust
  model, DoS guards
- `crates/replay/src/{main,role}.rs:1-100` — `ReplayRole`, `MAX_SERVERS`,
  `ReplayConfig`
- `crates/storage/src/{main,role,store}.rs` — `StorageRole`, SQLite store
- `crates/state/src/event.rs:54-95` — `Permission` enum, `SyncProvider`
- `docs/specs/2026-03-27-worker-nodes-design.md:160-310` — worker library
  design, four-actor concurrency, peer lifecycle, future worker types
- `docs/specs/2026-04-24-relay-capability-doc.md` — relay-capability
  precedent for the worker-capability-advertisement open question
- PR #636 (`/tmp/willow-pr-636.diff`) lines 421-444 — "Workers become
  generic peer hosts" + worker trust-model shift
- PR #636 lines 619-625 — child-spec list including "Worker as
  untrusted-WASM execution host"
- PR #636 lines 681-843 — maintenance-as-fourth-profile framing,
  Sybil/participation-enforcement open problem with prior-art survey
