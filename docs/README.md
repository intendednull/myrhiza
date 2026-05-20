# Myrhiza docs

Master index of Myrhiza's design specs, plans, reports, and prior-art studies. Grouped by area for discovery.

For build/test/dev commands and project framing, see [`../CLAUDE.md`](../CLAUDE.md). For the doc-organization conventions, see the [`organizing-docs` skill](../.claude/skills/organizing-docs/SKILL.md).

## Document types

- **[Specs](specs/)** — what we are building toward. Target shape of the code.
- **[Plans](plans/)** — how we get from current code to the target.
- **[Reports](reports/)** — one-shot investigations of our own codebase (audits, post-mortems).
- **[Prior art](prior-art/)** — deep-dive studies on external systems we learn from.
- **[References](references/)** — curated indices of papers + talks anchoring a topic. Single-file; no deep-dive (that's `prior-art/`).

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

- [Myrhiza master design](specs/2026-05-09-myrhiza-master-design/README.md) — runtime spec: three-tier architecture, four component profiles, event-log replay convergence, capability-mediated host surface, deterministic state-apply. Canonical reference for anything touching runtime semantics. `[draft]`
- [Plan B-1 — Event DAG + MemNetwork + cross-peer convergence](specs/2026-05-10-plan-b-1-dag-memnet-design.md) — per-author Merkle DAG, HeadsSummary sync, PendingBuffer, TUTTI-shaped drift detection, async Network trait. Realized by [plan B-1](plans/2026-05-10-plan-b-1.md). `[landed]`
- [Plan B-2.1 — Runtime perf carryovers (Q-1 + Q-7)](specs/2026-05-20-plan-b-2-1-perf-carryovers-design.md) — tip-fast-path replay optimization + `compute_anchor_digest` off-loop via `spawn_blocking`. Closes the two `TODO(B-2)` markers B-2 deferred. `[landed]`
- [Plan B-4.0 — Iroh transport skeleton](specs/2026-05-20-plan-b-4-0-iroh-skeleton-design.md) — pinned iroh 1.0.0-rc.0 + iroh-gossip 0.99.0 behind `network-iroh` feature; `IrohNetwork` struct + `Network` impl skeleton returning `NetError::Unimplemented`. First of a 4-slice B-4 sequence (skeleton → gossip → Q-4 → real-network tests). `[landed]`
- [Plan B-4.1 — Real subscribe + publish via iroh-gossip](specs/2026-05-20-plan-b-4-1-iroh-gossip-design.md) — wires `IrohNetwork::subscribe` / `publish` to `iroh_gossip::Gossip::subscribe` + `GossipSender::broadcast`. Adds `bootstrap: Vec<PeerPubkey>` parameter to the `Network::subscribe` trait. Defers Q-4 sender attribution to B-4.2 (gossip's `delivered_from` is last-hop, not original publisher). `[draft]`

### Networking & sync

*(transport, gossip, blob distribution, peer discovery)*

_(empty)_

### Identity, crypto & trust

*(keys, MLS, capability custody, peer auth)*

- [Plan B-2 — Persistent identity + B-1 carryover cleanup](specs/2026-05-19-plan-b-2-persistent-identity-design.md) — bech32m-encoded `PeerKeypair` + `AuthorKeypair` filesystem store, `IdentityStore` trait, plus carryover fixes Q-4 (pending-peer attribution) and N-12 (handle_heads_summary refactor) from B-1 review. `[draft]`
- [MLS / OpenMLS](prior-art/mls/) — IETF Standards Track group key agreement protocol (RFC 9420, July 2023) plus OpenMLS Rust implementation (`openmls 0.8.1`, MIT). 13 files, ~1,560 lines. CGKA-based; O(log N) on member updates; FS + PCS; production-shipping at Wire (RFC 9420 GA April 2025), Webex (on draft, migrating), Discord DAVE (RFC 9420, A/V-only since Sept 2024), Google RCS UP 3.0 (limited rollout 2026). NOT used by Apple iMessage (uses PQ3), WhatsApp, Signal, or Matrix. Surfaces critical Cremers ETK 2025 finding (MLS fails FCGKA with EUF-CMA-only signatures like ECDSA — published-RFC-level flaw; use Ed25519 not ECDSA). OpenMLS does not ship as a WASM Component Model artifact; Myrhiza will need to author the WIT contract for `MlsGroup` operations. The reference cryptographic primitive if Myrhiza grows multi-party room-shaped capabilities (channels, group state-apply, multi-party caps with rotating membership). Consult before any spec on group caps, multi-party auth, or post-quantum migration. `[active]`

### App distribution

*(component bundling, hashing, versioning, signing, install UX)*

_(empty)_

### Browser viability

*(jco transpile, sync-ABI submit-and-poll, host-import shims)*

_(empty)_

### Testing & tooling

*(test tiers, harnesses, dev workflow)*

_(empty)_

## References

Curated indices of papers + talks anchoring a topic. Single-file; companion to `prior-art/` deep dives.

- [Local-first + foundational references](references/local-first.md) — anchor index of papers and talks for Myrhiza's design space: Local-First essay (Kleppmann et al. 2019), Mark Miller's *Robust Composition* thesis (2006), Hewitt actor model (1973), Lamport time/ordering (1978), Shapiro CRDT survey (2011), Smith/Kay et al. Croquet (2003), Kleppmann move-tree (2021), Gentle & Kleppmann Eg-walker (2024), YATA (2016), Peritext (2022), Fugue (2023), RFC 9420 MLS, Cremers ETK 2025, Lin Clark Component Model talks. Reading order for new Myrhiza spec authors. `[active]`

## Prior art

External systems we learn from. Living documents — update on revision, archive when no longer worth tracking. Each system has its own subfolder; categories below are organizational only.

### P2P runtimes

- [Holochain](prior-art/holochain/) — peer-symmetric Rust runtime hosting WASM apps with deterministic-validation DHT. Closest architectural neighbor; consult when designing capabilities, determinism, networking, or identity. `[active]`
- [Pears / Holepunch](prior-art/pears/) — JavaScript-based consumer-mobile P2P stack: Hypercore (signed append-only log) + Hyperswarm (DHT + UDP holepunching, Noise-IK) + Bare (mobile-embeddable JS runtime) + Pear runtime + Keet messenger (closed-source flagship; iOS+Android, low-tens-of-thousands MAU). 17 files, ~3,550 lines. Single-vendor stewardship by Holepunch Inc, Tether-funded. Mixed MIT (Dat-era cores) / Apache-2.0 (Holepunch-era). The closest existing-art for state-apply event log substrate (Hypercore = append-only signed) and consumer-mobile P2P UX (Keet's iOS push-relay, suspend/resume). Not WASM — design lessons only, no API commitments. `[active]`
- [Spritely Goblins / OCapN](prior-art/spritely-ocapn/) — distributed object-capability runtime + cross-implementation network protocol (with Agoric, MetaMask, Cap'n Proto). Closest semantic neighbor on capability discipline; consult when designing the cap layer, distributed GC, sturdyrefs, promise pipelining, or netlayer abstraction. `[active]`
- [Agoric / Endo / SwingSet](prior-art/agoric-endo/) — production-hardened ocap + deterministic-replay JavaScript runtime; Cosmos chain since 2022-10-27, MetaMask Snaps in production at scale. Cousin to Spritely on the E lineage; the load-bearing reference for our `state-apply` purity, vat-snapshot/replay, computron metering, and the bundle-hash story. Consult before any spec on determinism, component upgrade, distributed GC, or app bundling. `[active]`
- [Willow](prior-art/willow/) — **internal architectural ancestor** of Myrhiza. P2P Discord replacement (Rust + Leptos + iroh + Ed25519 + ChaCha20-Poly1305 + X25519 + HLC), 15 production crates, ~30 specs, ~38 plans, single-author project (@intendednull, [github.com/intendednull/willow](https://github.com/intendednull/willow)). 16 files, ~3,800 lines. Captures the per-author Merkle DAG event-sourcing model, actor framework + lock-discipline, dual-target native+WASM compilation discipline, iroh trait abstraction (`Network`/`TopicHandle`/`BlobStore`), test-tier hierarchy, and PR #636's draft master-runtime spec — the *proto-spec for Myrhiza*. The four-component-profile table (`state-apply` strict / `state-propose` loose / `interaction` / `behavior`), pre-check-equals-apply mechanic, capability-only host surface, and key-handles-not-bytes custody story were lifted from PR #636 directly into Myrhiza CLAUDE.md. Open problems Willow surfaced (distributed maintenance + Sybil-resistant participation, multi-device identity, MLS adoption shape, hot-reload, snapshot portability) are inherited as Myrhiza's. Framing disclosure: written from a "Myrhiza-as-generalization-of-Willow" stance — not a neutral catalog. Consult before any spec on `state-apply` ABI, kernel capability surface, key custody, sync protocol, actor topology, worker security model, UI app contract, distributed maintenance, or MVP demo app shape. `[active]`

### Networking substrate

- [Iroh](prior-art/iroh/) — Rust P2P stack from Number 0: dial-by-pubkey QUIC, content-addressed blobs, NAT traversal via DERP-derived relays. **Load-bearing dependency** Myrhiza is committing to as transport substrate; consult before any kernel-network-cap, app-bundle-distribution, or peer-identity spec. `[active]`

### WASM platforms

- [WASM Component Model](prior-art/wasm-component-model/) — Bytecode-Alliance-stewarded substrate Myrhiza is committing to as foundation: the Component Model spec + WIT IDL + Canonical ABI + Wasmtime reference runtime + tooling (wasm-tools, wit-bindgen, cargo-component, jco, componentize-js, componentize-py, wac, wkg). 15 files, ~2,650 lines. WASI 0.2.11 stable; preview3 in RC since 2026-01. **Load-bearing dependency**; consult before any spec on kernel-import/host-capability surface, ABI/canonical-lift-lower, component bundling, browser viability, or determinism. `[active]`
- [wasmCloud](prior-art/wasmcloud/) — CNCF-Incubating production CM runtime built on Wasmtime; mid-pivot from v1 lattice-on-NATS to v2 K8s-native (`v2.0.0` 2026-03-22). 15 files, ~2,260 lines. Closest existing-art for Myrhiza's kernel-mediated capability model — the **v1 architecture** (capability providers + link definitions + wadm) is more relevant precedent than v2's K8s pivot. wRPC (BA-stewarded) is the cross-host RPC layer. Cosmonic Inc primary commercial steward; pivoted 2025-07 to "Cosmonic Control" K8s control plane. Consult before specs on host plugins, capability registration, link-revocation semantics, or component-bundle deployment. `[active]`
- [Spin (Akamai, formerly Fermyon)](prior-art/spin/) — request-driven serverless WASM CM runtime built on Wasmtime; sister CM runtime to wasmCloud at the *opposite* design point (Spin: stateless, per-trigger; wasmCloud: long-running, lattice/K8s-orchestrated). 11 files, ~1,470 lines. Apache-2.0 WITH LLVM-exception; `v4.0.0` (2026-04-20) on Wasmtime 43.0.1. Both Spin and SpinKube CNCF Sandbox (accepted 2025-01-21). **Fermyon acquired by Akamai 2025-12-01**; co-founders Matt Butcher + Radu Matei joined Akamai's Cloud Technology Group; Akamai committed to continuing Spin + SpinKube as open-source CNCF projects. Bus factor risk: 9 of 10 top contributors are Fermyon/Akamai (single-corporate-steward, structurally less resilient than wasmCloud's multi-vendor Incubating posture). Patterns directly borrowable for Myrhiza: SIP-021 factor architecture (per-host-capability runtime modules), SIP-023 fine-grained capability inheritance, manifest-static `spin.toml` capability declaration, OCI artifacts + `wkg` distribution, componentize-* build paths, `wac` build-time composition. Spin's request-driven shape is the *opposite* of Myrhiza's `state-apply` purity, so the borrow boundary is at the pattern level, not the component level. Consult before any spec on per-capability runtime modules, app manifest format, or component distribution. `[active]`
- _(future candidates: Extism — plugin-runtime niche; not direct CM substrate)_

### Sync protocols

- [CRDTs (Automerge + Yjs + Loro)](prior-art/crdts/) — multi-library survey of the three production-grade open-source CRDT libraries Myrhiza could build `state-apply` convergence on top of. 13 files, ~1,640 lines. Per-library deep dives (Automerge: Rust + Ink & Switch + RGA + Peritext; Yjs: pure-JS + bus-factor 1 + YATA + largest editor ecosystem; Loro: Rust-native + Fugue + Moveable Tree + bus-factor 1 + no at-scale users). Cross-cutting files cover CRDT theory (Treedoc → Logoot → WOOT → RGA → YATA → Fugue → Eg-walker lineage), history 2006-2026, ecosystem (Notion-uses-Yjs explicitly debunked), governance (bus-factor analysis), comparisons, open problems (what no CRDT solves: schema migration, authority, validation), critiques (Kleppmann's Eg-walker pivot, Boodman/Rocicorp), and lessons (validates / avoid / borrow + recommendation matrix). All three are MIT; none ship as WASM Component Model artifacts. Consult before any spec on `state-apply` convergence semantics, deterministic-merge ABI, or schema-migration story. `[active]`
- _(future candidates: Willow protocol, Eg-walker / diamond-types deep-dive)_

### Determinism & lockstep

- [Croquet / Multisynq](prior-art/croquet/) — lockstep deterministic-VM collaboration paradigm; the canonical reference for "all peers run the same compute on identically-ordered messages." 11 files, ~1,350 lines. Lineage: academic Croquet Project (Smith/Kay/Raab/Reed, C5 2003, originally Squeak Smalltalk) → Croquet Corporation (founded May 2018, $2.7M seed Feb 2020) → Multisynq Network (2024 open-source rebrand). Modern stack: `@multisynq/client` 1.1.0 (npm, Apache-2.0); `@croquet/croquet` 2.0.4 (npm, Apache-2.0 since 2025-06-09 republish — earlier versions proprietary). Synchronizer (reflector) ships as **closed-source Docker image** requiring a Synq Key issued by Multisynq — the SDK is open but the network is not. Bus-factor signal: Chief Architect Vanessa Freudenberg (JS-rewrite lead) died 2025-10-22; small team funded at $2.2M Multisynq seed. Lessons borrowed for Myrhiza: pseudo-time / virtual-time discipline, seeded RNG keyed to event, snapshot-equality voting (TUTTI pattern via `fast-json-stable-stringify`), `@stdlib/math` transcendental hardening with documented iOS-Safari `pow()` workaround, code-hash session scoping, forbidden-APIs-in-Models discipline. Lockstep is the **wrong primary `state-apply` shape for Myrhiza** (reflector dependency, scale ceiling, offline-intolerance) but the deterministic-VM mechanics translate cleanly. Closes the four-paradigm convergence survey alongside agoric-endo/ (event-log replay), crdts/ (merge), holochain/ (validating DHT). Consult before any spec on deterministic-VM mechanics, virtual-time abstraction, or cross-replica drift detection. `[active]`
