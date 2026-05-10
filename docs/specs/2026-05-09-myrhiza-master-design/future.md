**Date:** 2026-05-09
**Status:** draft
**Parent:** [README.md](README.md)
**Subject:** Myrhiza master design — Future-direction items


## 17. Future-direction items (named-but-deferred)

The master spec commits the *direction* on these items so v1 design
does not paint corners. Implementation lands in child specs when
demand emerges.

### Scaling
- Event-log replay scales linearly. Likely v2+ evolution: DHT-shape
  sharding layered on top. Other paths preserved (cooperative
  pinning, log-pruning, derived-state replication). Decision criteria:
  measure the bottleneck before committing.

### Distributed maintenance
- Default-instantiation heuristic for cheap-vs-expensive maintenance
  modules.
- Capability advertisement (peer signals "willing/able to host module
  X") — operator-config at v1; in-band gossip future.
- Resource limit defaults.
- Fair-share scheduling between topics on a single peer.
- Bridge between operator-deployed-infrastructure and social-graph
  invitation discipline.

### Identity
- Multi-device device-add/revoke flow.
- Recovery semantics (lost device).
- Cross-peer behavior continuity.
- Quantum-safe signature migration.

### Crypto
- `myrhiza-crypto-mls` module (when first MLS-needing app emerges).
- Other crypto modules (channel-key, double-ratchet, sealed-content).
- Quantum-safe primitives.

### Capability model
- High-value-op list for per-call gating.
- Cross-app authority composition (out of scope at v1).
- Capability vocabulary in manifest schema.

### Distribution
- Bundle revocation (author retracts bad version).
- In-band catalog gossip for app/module discovery.
- Supply-chain hardening (dependency review tooling).

### Networking
- Topic-ID rotation through dumb relays (relay-and-rotation child
  spec).
- `HistorySyncComplete` EOSE-style signal for backfill completion.
- Negentropy-shape range reconciliation for very large topics.

### Determinism
- Float opt-in path (manifest `state-apply.allow-floats = true`).
- Snapshot portability across component-version upgrades.
- Additional state-digest formats opt-in (bincode is pinned at v1;
  future opt-ins via manifest declaration).
- Pre-check fuel budget independence from apply.

### Interaction
- `ui:*` WIT contract details.
- Custom-pixel surface escape hatch on non-web platforms.
- Hot-reload (deferred to v2).

### Module ecosystem
- Versioning + semver discipline child spec (already content-hash
  pinned per [distribution.md](distribution.md) §10.6, but version-display + compatibility checks).
- Bus-factor on official `myrhiza-*` modules.
- Module audit / curation policy.

### Prior art borrowed but not yet implemented

Patterns from `prior-art/` that the master spec acknowledges and
commits as future-direction; implementation lands in child specs or
module ecosystem.

- **Holochain source-chain semantics** (`prior-art/holochain/lessons.md`
  Borrow §1) — already aligned: per-author Merkle DAG IS source-chain
  shape. No future work needed; called out for clarity.
- **Holochain DHT op decomposition** (`prior-art/holochain/lessons.md`
  Borrow §2) — informs v2+ scaling direction ([convergence.md](convergence.md) §4.5). Events
  decomposed into typed ops, sharded by neighborhood. v2 scaling
  child spec.
- **Holochain warrants** (`prior-art/holochain/lessons.md` Borrow [architecture.md](architecture.md) §3) —
  signed attestations of bad-author behavior (equivocation, etc.). v2
  warrant-and-equivocation child spec. Surfaced in [convergence.md](convergence.md) §4.4.1 future
  direction.
- **Holochain countersigning** (`prior-art/holochain/lessons.md`
  Borrow [convergence.md](convergence.md) §4) — multi-author atomic events. Relevant to governance
  modules; deferred. Possible v2 `myrhiza-permission-countersign`
  module.
- **Holochain membrane proofs** (`prior-art/holochain/lessons.md`
  Borrow [identity.md](identity.md) §6) — capability-bound app entry. Relevant to participation
  primitive; informs `myrhiza-permission-rbac` / `myrhiza-participation-*`
  module designs.
- **Croquet TUTTI snapshot-equality voting** (`prior-art/croquet/lessons.md`
  Borrow §"Snapshot-equality voting") — ratified in [convergence.md](convergence.md) §4.7 (cross-peer
  drift detection). Implementation lands at v1.
- **Agoric `baggage` upgrade convention** (`prior-art/agoric-endo/lessons.md`
  Borrow §"`baggage` upgrade convention") — durable component-state
  bridge across upgrades. Informs snapshot portability child spec.
- **Agoric `bringOutYourDead` distributed GC** (`prior-art/agoric-endo/lessons.md`
  Borrow §"`bringOutYourDead`") — long-lived peer-as-infra needs GC
  of stale state (event log, snapshot cache, per-component KV).
  Future-direction for distributed-maintenance child spec.
- **Willow `timestamp_hint_ms` split-semantics review-trap**
  (`prior-art/willow/lessons.md` Avoid) — Willow signs HLC into events
  but doesn't use it for ordering, only materialized-state. Myrhiza
  inherits this exactly ([convergence.md](convergence.md) §4.1). Pick-a-side mitigation: master spec
  documents both uses explicitly (HLC IS extracted via
  `host.now-hlc-from-event` and IS materialized into derived state;
  HLC is NOT used for DAG topo-sort or merge). Reduces but does not
  eliminate the review-trap; add static-analysis tooling future
  direction.


