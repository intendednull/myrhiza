**Date:** 2026-05-09
**Status:** active (preparation phase for master-spec brainstorming)
**Subject:** Myrhiza decision space — brainstorming preparation

# Myrhiza design-space exploration

Output of the preparation phase before the Myrhiza master-spec
brainstorming session. Captures **what's locked**, **what's open**,
**the options at each open point**, **tradeoffs across prior-art**, and
a **sorted brainstorming question list** ready to drive the next step.

This is the consumer-side application of the prior-art corpus
(`docs/prior-art/{willow, agoric-endo, croquet, crdts, holochain, iroh,
mls, pears, spin, spritely-ocapn, wasmcloud, wasm-component-model}/`).
Each cluster file mines a subset of the corpus for decision domains,
options, and Myrhiza re-evaluation questions.

## Files in this report

| File | Lines | Cluster |
|---|---|---|
| `README.md` (this file) | ~400 | Master index + consolidated question list |
| [`convergence-and-determinism.md`](convergence-and-determinism.md) | 287 | Convergence paradigm, state-apply discipline, determinism enforcement |
| [`wasm-and-abi.md`](wasm-and-abi.md) | 1425 | ABI choice, composition, WIT, bundle distribution, browser, instantiation, async, determinism+WASM |
| [`identity-crypto-caps.md`](identity-crypto-caps.md) | 600 | Identity custody, multi-device, capability discipline, crypto host imports, MLS, ocap |
| [`networking-sync-maintenance.md`](networking-sync-maintenance.md) | 914 | Gossip + blobs, relay model, sync protocol, worker trust, distributed maintenance, Sybil, fuel limits, browser peer story, topic-ID rotation |
| [`ui-distribution-mvp.md`](ui-distribution-mvp.md) | 601 | UI app contract, custom-pixel escape hatch, UI app catalog, multi-tenancy, app distribution, signing, browser-vs-native v1, MVP shape, Willow migration |

Total: ~3,800 lines of detailed mining across 12 prior-art folders + the
references index + Willow's PR #636 master runtime spec + Myrhiza's own
CLAUDE.md.

## How to use this report

1. **Read this README end-to-end** — it has the question list, the
   locked-vs-open decision split, and the cluster index.
2. **For any specific decision domain**, jump to the relevant cluster
   file via the table above.
3. **The brainstorming session** that follows uses this report as its
   working context. Questions are pre-sorted by criticality so the
   session can work top-down.

## What's already locked (don't re-litigate)

These decisions live in `/mnt/storage/projects/myrhiza/CLAUDE.md` or are
unanimously confirmed across prior-art clusters. Brainstorming should
**not** revisit them unless evidence emerges they were wrong.

- **The four-component-profile split** — `state-apply` (strict pure) /
  `state-propose` (loose) / `interaction` (non-deterministic, per-peer)
  / `behavior` (non-deterministic, per-(peer, instance)).
- **Strict `state-apply` purity** — pure function of `(prior state,
  event)` plus the deterministic helper set; no clock, no random, no
  network, no FS, no threads.
- **Pre-check is mechanically the same WASM function as `state-apply`**
  in dry-run mode. Not a convention. Failure-closed (exhausting fuel /
  trapping rejects the user's action; never admits an event every peer
  will reject at apply).
- **Capabilities are the only host surface.** Apps reach the host
  through declared imports. Adding a new host import is an ABI change.
- **Cross-peer convergence via app-exported `state-digest()`**, not raw
  WASM linear memory hash. Sorted-collection serialization discipline
  (`BTreeMap` / `BTreeSet`) is the load-bearing piece; format choice
  (bincode today / postcard envisioned / other) is open.
- **Determinism enforcement via absence of non-deterministic imports**
  (proof by construction). Plus instruction-count fuel (not wall time);
  spec-pinned floats with a strong recommendation to ban in v1.
- **iroh as transport** — gossip + content-addressed blobs +
  dial-by-pubkey. Locked load-bearing dep.
- **WASM Component Model semantics** as the *eventual* ABI shape. v1
  *implementation path* is open (Extism v1 vs full CM v1 vs hybrid).
- **Three-layer persistence** — event-log canonical / per-component KV /
  snapshot-as-cache. Every paradigm reaches this shape.
- **Peer-symmetric, no client/server distinction in the kernel.** A
  laptop UI, a worker, an MCP agent are all "kernel + a different mix
  of components."
- **Private signing keys live only in the kernel.** Components describe
  events; the kernel signs. Symmetric channel/group keys, ratchets,
  MLS group state are kernel-custodied via opaque app-declared key
  handles.
- **Apps as bundles of WASM components** — content-addressed, signed,
  fetched via iroh-blobs, lazy-loaded, hash-cached.
- **Don't bake a CRDT into the kernel.** Apps may use a CRDT inside
  their own `state-apply`; the kernel stays generic.
- **Willow stays separate**, develops independently, eventually
  refactors onto Myrhiza. Not a fork. Re-evaluate every Willow decision.

## Brainstorming question list — sorted by criticality

The 12 questions below are the load-bearing decisions for the master
spec. Questions 1-4 set the v1 shape and timeline; 5-8 set the
correctness substrate; 9-12 set the security and operating model.

Each question links to its full options table in the cluster file.

### 1. MVP demo app shape — what's the first app we ship?

**Why critical**: the MVP implicitly locks ABI choice, browser-vs-native,
and timeline. Counter or poll → Extism v1 in 4-6 weeks. Single-channel
non-Willow chat or kanban → full CM v1 in 3-4 months. The choice also
proves "kernel doesn't know about chat" (different from Willow's chat
shape) — or doesn't.

**Options**: shared-counter (~50 lines state, ~100 lines interaction);
single-channel non-Willow chat that doesn't reuse `ServerState`;
real-time poll; tiny kanban; tiny wiki; agent-readable demo for
`willow-ui-mcp`.

**Cluster file**: [`ui-distribution-mvp.md`](ui-distribution-mvp.md) §"MVP demo shape".

### 2. ABI choice — Extism v1 vs full Component Model v1 vs hybrid

**Why critical**: locks app-author migration cost, browser viability,
ecosystem alignment, ship date. Extism is faster but caps composition
expressiveness (no resource handles, no borrows, no world composition).
Full CM is slower but ecosystem-aligned. Hybrid (CM-shaped Extism
subset) tries both.

**Options**:
- **A. Full Component Model day-one** — wit-bindgen + Wasmtime native
  + jco browser. Ecosystem-aligned. Heavier toolchain. Browser CM
  still maturing; ~350KB JS shim floor; no async on browser side.
- **B. Extism v1, WIT-shaped subset, migrate to CM later** — ship
  faster on simpler runtime; every host-call signature WIT-expressible.
  Cross-component composition is **kernel-brokered RPC by opaque ID
  only**. Migration is a real refactor for app authors (not regen
  bindings).
- **C. Hybrid (CM-shaped Extism subset)** — Extism foundation but
  agree to WIT-derivable signatures from day one. Closer to A's
  migration target.
- **D. Native-only-no-WASM-v1** — kernel ships, apps are trusted Rust
  in v1, WASM later. Fastest to functional MVP. Defeats sandbox model.

**Cluster file**: [`wasm-and-abi.md`](wasm-and-abi.md) §"Domain 1: ABI choice".

### 3. Convergence paradigm — what's our cross-peer agreement model?

**Why critical**: determines `state-apply` ABI shape, sync protocol,
sharding model, offline tolerance, divergence-response. PR #636 picked
event-log replay without an explicit alternatives section; brainstorming
must validate or revise.

**Options**:
- **A. Event-log replay** (Agoric vat-snapshot/replay; Willow per-author
  Merkle DAG today). Strict-state-apply replay over canonical event
  order. Locked-in by inheritance from Willow.
- **B. Validating DHT** (Holochain). Peers validate shards; closest
  existing precedent for distributed peer-as-infra. Costs: validation
  per neighborhood, eclipse attacks, no formal verification.
- **C. CRDT merge** (Automerge / Yjs / Loro). Commutative ops, no
  replay. Doesn't solve schema migration, authority, validation.
  Corpus uniformly recommends "don't bake into kernel."
- **D. Lockstep deterministic VM** (Croquet). Reflector ordering, all
  peers run same compute. Reflector dependency conflicts with
  P2P-native commitment.
- **E. Hybrid** — event-log-replay-with-CRDT-typed-payloads or
  event-log-replay-with-DHT-sharding.

**Cluster file**: [`convergence-and-determinism.md`](convergence-and-determinism.md) §"Domain 1: Convergence paradigm".

### 4. Browser-first vs native-first at v1

**Why critical**: jco maturity is the largest schedule risk. Browser-first
forces sync-ABI submit-and-poll, ~350KB shim floor, no async — and
defers schedule by months waiting on jco preview3. Native-first defers
Willow refactor by ~1 year (Willow IS browser-shipped today).

**Options**:
- **A. Browser-first v1** — jco transpile + sync-ABI submit-and-poll.
  Aligns with Willow's existing surface but inherits jco immaturity
  risk.
- **B. Native-first v1, browser v2** — wasmtime-only at v1; browser
  comes when CM browser ABI matures (likely 2027). Willow refactor
  blocked on this.
- **C. Dual-stack v1** — both targets, kernel internals abstract over
  Wasmtime + jco backends. Heavier engineering, fastest path to all
  user surfaces.
- **D. WebView-only browser** — ship native-only, embed in Tauri /
  Wails / Electron for desktop. Skip jco entirely. Still no
  pure-browser deployment.

**Cluster file**: [`ui-distribution-mvp.md`](ui-distribution-mvp.md)
§"Browser viability" + [`wasm-and-abi.md`](wasm-and-abi.md) §"Domain 5: Browser viability".

### 5. Sybil-resistant participation primitive shape

**Why critical**: the load-bearing open problem from PR #636 research
notes. Without this, peer-as-infrastructure scales linearly with honest
peers + cheaters get free ride. The literature has 20+ years of
designs; we have to pick.

**Options**:
- **A. Tit-for-tat** (BitTorrent choking, IPFS Bitswap) — local
  pairwise reciprocity; Sybil-tolerant per-connection.
- **B. Reputation aggregation** (EigenTrust, Tribler BarterCast) —
  gossip eigenvector, Sybil-vulnerable without identity cost.
- **C. Social-graph Sybil resistance** (SybilGuard / SybilLimit /
  Whanau) — leverage Willow's existing permission/invite trust graph.
  **The unique Myrhiza advantage.**
- **D. DHT-responsibility** (Holochain). Coordinated allocation;
  closest existing precedent.
- **E. Storage proofs** (Filecoin / Storj / Sia). Heavy machinery;
  probably overkill for chat-shape but right for high-stakes data.
- **F. Hybrid** — most plausible (e.g. C as baseline + A for
  bandwidth-heavy roles).

**Cluster file**: [`networking-sync-maintenance.md`](networking-sync-maintenance.md)
§"Domain 6: Sybil-resistant participation".

### 6. Multi-device identity + behavior identity unified mechanism

**Why critical**: PR #636 names them as **structurally the same problem**
(long-term identity + short-lived per-instance signing key). MLS LeafNode
signing keys also fit. Three trust domains, one mechanism — or invent
three times.

**Options**:
- **A. Unified `IdentityScope { long_term, instance: (peer, kind, name) }`**
  with kernel as custodian. Both multi-device and behavior get the same
  primitive. MLS LeafNode keys also fit.
- **B. Separate mechanisms per domain** (Willow today). Multi-device
  deferred; behavior keypair-per-(peer, instance) (PR #636).
- **C. Kernel-built-in-multi-device, behavior-app-defined**. Asymmetric.
- **D. Defer all three to v2.**

**Cluster file**: [`identity-crypto-caps.md`](identity-crypto-caps.md)
§"Domain 2: Multi-device + behavior identity".

### 7. Worker trust model — WASM-untrusted vs Rust-trusted

**Why critical**: PR #636 envisions workers as generic peer hosts running
attacker-influenceable third-party WASM. Willow today runs trusted Rust
workers. Switching means new responsibilities: DoS resistance, fuel
scheduling, fair-share between topics, operator deny-lists. Maintenance
components (4th profile per PR #636 research notes) sit at this seam.

**Options**:
- **A. WASM-untrusted from day one** (PR #636 envisioned). Workers run
  third-party state components in sandboxes. Adds fuel scheduler +
  per-instance memory caps + operator deny-lists.
- **B. Rust-trusted in v1, WASM in v2** (Willow today's model
  preserved). Faster v1 ship; security model regression to fix later.
- **C. Hybrid — kernel-mediated workers run trusted Rust adapters
  that exec untrusted WASM**. Belt-and-suspenders.
- **D. No workers in v1** — peers self-host all maintenance; defer
  worker model to v2.

**Cluster file**: [`networking-sync-maintenance.md`](networking-sync-maintenance.md)
§"Domain 4: Worker trust model".

### 8. Cross-component composition — typed resources vs opaque IDs

**Why critical**: Extism's hard limitation (no resource handles, no
borrows, no world composition) becomes app-author-visible cost at the
v1 ABI choice. Composition shape determines whether two components can
hold non-forgeable references to each other's objects.

**Options**:
- **A. Typed resource handles** (full CM, wit-bindgen). Components hold
  non-forgeable refs. Borrows replace clone-and-pass. Composition is
  expressive but requires CM mature enough on browser.
- **B. Opaque IDs** (Extism / kernel-broker). Components pass
  string/u64; kernel resolves. Simple. Loses non-forgeability; relies
  on kernel as cap arbiter.
- **C. Message-passing only** (Spritely / Erlang shape). No shared
  state, no shared refs; everything is a typed message. Cleanest
  capability discipline.
- **D. Shared memory** — explicitly rejected per PR #636 (no direct
  memory-shared linkage between components).

**Cluster file**: [`wasm-and-abi.md`](wasm-and-abi.md) §"Domain 2: Composition".

### 9. MLS adoption shape

**Why critical**: load-bearing for group encryption, multi-party caps,
post-quantum migration. Willow defers MLS to a future spec; PR #636
commits the placement (`host.mls` capability, kernel-side MLS engine)
without committing the API. Cremers ETK 2025 (must use Ed25519 not
ECDSA) is non-negotiable.

**Options**:
- **A. MLS in v1**, kernel ships OpenMLS-equivalent (or the existing
  Rust `openmls` crate). Apps emit Welcome/Commit/Application via
  state propose; kernel processes.
- **B. MLS in v2**, ship simpler symmetric-channel-key model first
  (Willow's current `seal_content`).
- **C. Defer indefinitely** — chat-shape apps don't need MLS-grade
  security; MLS adopters self-implement on top.
- **D. Hybrid — MLS *bindings* in v1 (the WIT contract is stable),
  *implementation* deferred** (kernel returns "not implemented" until
  v2). Forces apps to design against the eventual API.

**Cluster file**: [`identity-crypto-caps.md`](identity-crypto-caps.md)
§"Domain 6: MLS adoption shape".

### 10. App distribution + signing trust root

**Why critical**: how users install apps + how signatures verify.
PR #636 commits to iroh-blobs hash-pinning; signing model is open.
Spin/wasmCloud use OCI artifacts + cosign/sigstore. Myrhiza is
P2P-native (no central registry).

**Options**:
- **A. Ed25519-over-manifest-hash, author-self-signed**, distributed
  via iroh-blobs. P2P-native; no central registry; web-of-trust
  signing.
- **B. OCI artifacts via `wkg`-equivalent registry**, cosign/sigstore
  verification. Industry-standard; centralizes registry.
- **C. Hybrid** — iroh-blobs distribution + OCI-style signing format
  (sigstore bundles distributed via blobs).
- **D. No signing in v1** — install-by-hash only; trust comes from
  out-of-band sharing.

**Cluster file**: [`ui-distribution-mvp.md`](ui-distribution-mvp.md)
§"App distribution + signing".

### 11. Per-call vs per-import-binding capability gating

**Why critical**: PR #636 commits to per-call gating on `ui:*` privileged
surfaces (clipboard, file picker, push registration each gated by the
*calling component's* manifest). Spritely's full ocap discipline is
stricter (every reference IS a capability; no ambient authority).

**Options**:
- **A. Per-call gating** (PR #636 default). Manifest-declared imports
  define the ambient scope; specific privileged calls re-check the
  caller's manifest.
- **B. Full ocap discipline** (Spritely). No ambient authority within
  declared scope; every capability is an explicit handle.
- **C. Hybrid** — ambient for cheap calls, ocap for high-value ones.

**Cluster file**: [`identity-crypto-caps.md`](identity-crypto-caps.md)
§"Domain 3: Capability discipline".

### 12. Float discipline + WASM determinism details

**Why critical**: WASM spec pins float behavior, but cross-platform NaN
canonicalization and SIMD float operations have known divergence vectors.
PR #636 leans toward banning floats in `state-apply` v1.

**Options**:
- **A. Ban floats from state-apply** (PR #636 lean). Simplest. App
  authors must use scaled integers for any numeric.
- **B. Allow spec-pinned floats** but ban SIMD; lint for canonical-NaN.
- **C. Allow with audit** — accept divergence risk, hash mismatch
  surfaces bugs.

**Cluster file**: [`convergence-and-determinism.md`](convergence-and-determinism.md)
§"Domain 3: Determinism enforcement".

## Cross-question entanglements

Some questions can't be answered independently. The brainstorming session
should resolve these in clusters:

- **Q1 (MVP) ↔ Q2 (ABI) ↔ Q4 (browser)** — MVP shape locks ABI choice
  locks browser strategy. Decide together.
- **Q3 (convergence) ↔ Q5 (Sybil) ↔ Q7 (workers)** — paradigm choice
  affects sharding affects participation primitive affects worker
  responsibility.
- **Q6 (identity) ↔ Q9 (MLS) ↔ Q11 (capability)** — identity
  unification shape constrains MLS adoption shape constrains capability
  gating mechanism.
- **Q2 (ABI) ↔ Q8 (composition)** — full CM enables typed resource
  handles; Extism forces opaque IDs.
- **Q5 (Sybil) leverages permission/invite trust graph** which exists
  only because Willow already implemented the chat-app authority model.
  Is that pattern guaranteed to be in every Myrhiza app, or only the
  apps that opt in?

## Open questions that didn't make the top 12

These are real but lower-leverage. They'll surface naturally during the
brainstorming session or land in child specs.

- Topic-ID rotation through dumb relays without leaking next-topic IDs
  publicly (cluster D).
- WIT contract granularity for `host.mls.*` (coarse / medium / fine)
  (cluster C).
- Per-app fuel + memory budget defaults (cluster D).
- Snapshot portability across component-version upgrades (cluster A).
- Resource handle / namespace ownership rules across multi-tenant apps
  (cluster E).
- Custom-pixel surface escape hatch for non-web (TUI / mobile-native)
  surfaces (cluster E).
- App discovery — out-of-band install vs in-band publish (cluster E).
- Hot-reload story (deferred to v2 in PR #636) — confirm or revisit.
- Cross-app authority composition (deferred to v2 in PR #636) — confirm.
- Pre-check fuel budget independence from apply (cluster A).
- Behavior coordination primitives (kernel vs app) (cluster D).

## Already-rejected paradigms (cite-and-move)

These were considered and rejected by the corpus. Don't re-litigate
unless evidence changes.

- **Lockstep deterministic VM as primary** — reflector dependency
  conflicts with P2P-native commitment (cluster A; `prior-art/croquet/`).
- **CRDT baked into kernel** — kernel stays generic; apps may CRDT
  internally (cluster A; `prior-art/crdts/lessons.md`).
- **Pears stack (Hypercore + Hyperswarm + Bare)** — JS-on-native
  libsodium, no WASM build, protomux head-of-line blocking, wire-incompat
  version cuts (cluster D; `prior-art/pears/`). Conceptual lessons
  inherited; specific stack rejected.
- **Bevy as primary UI substrate** — kept as far-future GPU escape
  hatch only (cluster E; `prior-art/willow/ui.md`).
- **Shared memory between components** — explicitly rejected by PR #636
  (cluster B).
- **Transparent distributed RPC** — wasmCloud v1 → v2 lesson is "this
  was a footgun" (cluster B; `prior-art/wasmcloud/`).

## Process: how the brainstorming session will use this

1. **Confirm locked-in decisions** (the §"What's already locked"
   section). Quick scan; flag anything that should be re-opened.
2. **Walk top 12 questions in criticality order**. Each question gets
   a decision (or an explicit "defer to child spec X").
3. **Resolve entanglement clusters together** (§"Cross-question
   entanglements").
4. **Confirm rejected paradigms still rejected** (§"Already-rejected
   paradigms").
5. **Capture decisions + rationale** as input to the master spec at
   `docs/specs/2026-05-09-myrhiza-master-design.md`.

The brainstorming session is the next step. Master-spec authoring
follows the brainstorming output; this report is the input to
brainstorming, not to the master spec directly (though many of these
questions become master-spec sections).

## Sources

- All 12 prior-art folders under `/mnt/storage/projects/myrhiza/docs/prior-art/`.
- Willow master runtime spec PR #636: [github.com/intendednull/willow/pull/636](https://github.com/intendednull/willow/pull/636); local diff at `/tmp/willow-pr-636.diff`.
- Willow repo: [github.com/intendednull/willow](https://github.com/intendednull/willow).
- Myrhiza CLAUDE.md: `/mnt/storage/projects/myrhiza/CLAUDE.md`.
- References anchor: `docs/references/local-first.md`.
- This report's cluster files (5 above).
