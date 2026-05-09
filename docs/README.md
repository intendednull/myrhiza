# Myrhiza docs

Master index of Myrhiza's design specs, plans, reports, and prior-art studies. Grouped by area for discovery.

For build/test/dev commands and project framing, see [`../CLAUDE.md`](../CLAUDE.md). For the doc-organization conventions, see the [`organizing-docs` skill](../.claude/skills/organizing-docs/SKILL.md).

## Document types

- **[Specs](specs/)** — what we are building toward. Target shape of the code.
- **[Plans](plans/)** — how we get from current code to the target.
- **[Reports](reports/)** — one-shot investigations of our own codebase (audits, post-mortems).
- **[Prior art](prior-art/)** — deep-dive studies on external systems we learn from.

## Status tags

Specs/plans/reports carry one of:

- `[draft]` — being written, target not yet stable.
- `[active]` — current target / in-flight migration.
- `[landed]` — realized in code; canonical reference.
- `[superseded]` — replaced; entry links to successor.

Prior-art docs carry `[active]` or `[archived]`.

## Catalog

### Runtime core

*(specs, plans, and reports for the kernel: component model, capabilities, state-apply ABI, lazy loading, cross-peer convergence)*

_(empty — see incoming runtime master spec)_

### Networking & sync

*(transport, gossip, blob distribution, peer discovery)*

_(empty)_

### Identity, crypto & trust

*(keys, MLS, capability custody, peer auth)*

_(empty)_

### App distribution

*(component bundling, hashing, versioning, signing, install UX)*

_(empty)_

### Browser viability

*(jco transpile, sync-ABI submit-and-poll, host-import shims)*

_(empty)_

### Testing & tooling

*(test tiers, harnesses, dev workflow)*

_(empty)_

## Prior art

External systems we learn from. Living documents — update on revision, archive when no longer worth tracking. Each system has its own subfolder; categories below are organizational only.

### P2P runtimes

- [Holochain](prior-art/holochain/) — peer-symmetric Rust runtime hosting WASM apps with deterministic-validation DHT. Closest architectural neighbor; consult when designing capabilities, determinism, networking, or identity. `[active]`
- [Spritely Goblins / OCapN](prior-art/spritely-ocapn/) — distributed object-capability runtime + cross-implementation network protocol (with Agoric, MetaMask, Cap'n Proto). Closest semantic neighbor on capability discipline; consult when designing the cap layer, distributed GC, sturdyrefs, promise pipelining, or netlayer abstraction. `[active]`
- [Agoric / Endo / SwingSet](prior-art/agoric-endo/) — production-hardened ocap + deterministic-replay JavaScript runtime; Cosmos chain since 2022-10-27, MetaMask Snaps in production at scale. Cousin to Spritely on the E lineage; the load-bearing reference for our `state-apply` purity, vat-snapshot/replay, computron metering, and the bundle-hash story. Consult before any spec on determinism, component upgrade, distributed GC, or app bundling. `[active]`

### Networking substrate

- [Iroh](prior-art/iroh/) — Rust P2P stack from Number 0: dial-by-pubkey QUIC, content-addressed blobs, NAT traversal via DERP-derived relays. **Load-bearing dependency** Myrhiza is committing to as transport substrate; consult before any kernel-network-cap, app-bundle-distribution, or peer-identity spec. `[active]`

### WASM platforms

- [WASM Component Model](prior-art/wasm-component-model/) — Bytecode-Alliance-stewarded substrate Myrhiza is committing to as foundation: the Component Model spec + WIT IDL + Canonical ABI + Wasmtime reference runtime + tooling (wasm-tools, wit-bindgen, cargo-component, jco, componentize-js, componentize-py, wac, wkg). 15 files, ~2,650 lines. WASI 0.2.11 stable; preview3 in RC since 2026-01. **Load-bearing dependency**; consult before any spec on kernel-import/host-capability surface, ABI/canonical-lift-lower, component bundling, browser viability, or determinism. `[active]`
- [wasmCloud](prior-art/wasmcloud/) — CNCF-Incubating production CM runtime built on Wasmtime; mid-pivot from v1 lattice-on-NATS to v2 K8s-native (`v2.0.0` 2026-03-22). 15 files, ~2,260 lines. Closest existing-art for Myrhiza's kernel-mediated capability model — the **v1 architecture** (capability providers + link definitions + wadm) is more relevant precedent than v2's K8s pivot. wRPC (BA-stewarded) is the cross-host RPC layer. Cosmonic Inc primary commercial steward; pivoted 2025-07 to "Cosmonic Control" K8s control plane. Consult before specs on host plugins, capability registration, link-revocation semantics, or component-bundle deployment. `[active]`
- _(future candidates: Spin, Extism — production CM runtimes built on Wasmtime substrate)_

### Sync protocols

_(empty — Willow protocol, Automerge candidates; iroh-docs covered under Iroh folder)_

### Determinism & lockstep

_(empty — Croquet/Multisynq candidate)_
