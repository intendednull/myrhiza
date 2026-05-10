**Date:** 2026-05-09
**Status:** active (running log; updated as brainstorming session progresses)
**Subject:** Myrhiza master-spec brainstorming session — decisions + rationale

# Brainstorming decisions log

Running record of decisions made during the master-spec brainstorming
session. Each entry: question, options considered, choice, rationale,
runner-up + why-rejected, future-evolution path (if applicable),
forces-downstream summary.

This file feeds directly into `docs/specs/2026-05-09-myrhiza-master-design.md`
when the brainstorm is complete.

## Q3 — Convergence paradigm

**Decision: A. Event-log replay**

Per-author signed Merkle DAG. Each event has author, prev (own chain),
deps (cross-author causal), opaque payload. Peers gossip events,
topo-sort, replay through `state-apply`. Pure determinism + canonical
event order = cross-peer convergence.

**Rationale:**

- **Production-validated.** Agoric runs Cosmos chain on this since
  2022-10-27. Willow ships chat product on this. Vat-snapshot/replay is
  well-understood at scale.
- **Strict state-apply purity maps cleanly.** Pure function over events
  fits the four-profile model already locked in CLAUDE.md.
- **Audit-friendly.** Event log = full causal history. Convergence-bug
  debugging = re-run the log.
- **Snapshot model is straightforward.** Snapshots = cached
  materialization, not authoritative.
- **Willow → Myrhiza migration is mechanical.** Willow's `EventDag` +
  `materialize` + `HeadsSummary` generalize directly. Substrate work
  doesn't restart.
- **Low schedule risk for v1.** Familiar shape; engineers and tooling
  already exist.

**Runners-up + why rejected at v1:**

- **B. Validating DHT (Holochain).** Closest precedent for peer-as-infra
  scaling, but Holochain's sharding story is ~6 years unfinished. We
  don't have years to spend on substrate work. Eclipse + collusion
  attacks are real, mitigations incomplete. state-apply purity model
  doesn't translate cleanly (validation function ≠ apply function).
  Rejected at v1; revisit if A's scaling fails before alternatives
  prove out.
- **C. Event-log + DHT-sharding hybrid.** *Not rejected.* Reframed as
  the **most-likely v2 evolution path** if scaling demand emerges.
  Documented as future direction (see below). Premature at v1: adds
  doc surface without functional v1 difference.
- **D. Event-log + per-app CRDT-in-state-apply hybrid.** *Not rejected.*
  Reframed as a **documented usage pattern** for apps that need
  commutative merge inside their own state-apply. Kernel stays generic
  (`prior-art/crdts/lessons.md` consensus). Document when first
  CRDT-shaped app lands; no kernel work required.

**Already-rejected paradigms (cite-and-move; do not re-litigate):**

- **Lockstep deterministic VM (Croquet).** Reflector dependency
  conflicts with P2P-native commitment. Useful determinism mechanics
  inherited (pseudo-time, snapshot-equality voting), not the
  architecture.
- **CRDT in kernel.** Corpus consensus: kernel stays generic; apps may
  CRDT internally. See `prior-art/crdts/lessons.md`.

## Future direction: scaling problem (open path)

**The scaling problem is acknowledged, not yet committed to a solution.**

Event-log replay scales linearly: every materializing peer carries the
full log for the topic. Big apps (large peer counts, large state, public
read access) hit this ceiling.

**Most-likely v2 answer**: option C — DHT-sharding layered on top of
event-log replay. Maintenance components (PR #636 4th profile —
persister, snapshot provider, sync provider, replay buffer) become DHT
roles naturally. Closest existing precedent: Holochain's
DHT-responsibility model (with caveats about its unfinished story).

**Other potential answers worth exploring before committing**:

- **Author-bounded scale only at v1.** Many apps don't need
  Twitter-scale; chat-shape, kanban-shape, wiki-shape with bounded
  authors fit fine on event-log replay alone. The scaling problem may
  not be Myrhiza's problem for the first generation of apps.
- **Snapshot-as-bootstrap with log-pruning.** Eg-walker style log
  compaction (research-grade as of 2026); log truncation past a snapshot
  is well-understood for some shapes (Agoric does this for vat
  upgrades).
- **Cooperative pinning.** Maintenance components opt-in; apps that
  need durability ship with persistence-component bundles; users who
  install run them. Closer to BitTorrent's tit-for-tat than DHT
  responsibility. Lighter than full sharding.
- **Hybrid: event-log canonical, derived state replicated through a
  separate channel.** Read replicas materialize from log on dedicated
  workers, gossip the materialized state directly. Some apps are
  read-heavy enough that this dominates the cost.

**Commitment for v1 master spec**: ship A. Document the scaling problem
as named-but-deferred. Reserve hooks (subscription model, blob
distribution patterns, maintenance-component slots) such that v2 can
add C (DHT-sharding) or alternative without re-architecting v1 apps.

**Decision criteria for picking the v2 answer**: when the first Myrhiza
app hits the scaling ceiling, measure where the actual bottleneck is
(storage cost / replay time / bandwidth / participation enforcement) and
pick the answer that addresses *that* bottleneck. Don't speculatively
ship sharding before the bottleneck is real.

## Forces ratified by Q3 (downstream consequences)

- ✅ Per-author signed Merkle DAG + topo-sort + state-apply pure replay
  is the substrate.
- ✅ HeadsSummary-style delta sync is the wire protocol shape.
- ✅ Snapshot = cached materialization, not authoritative.
- ✅ Willow → Myrhiza migration is mechanical (no paradigm change).
- ⚠️ Q5 (Sybil-resistant participation) is its own problem, not solved
  by paradigm choice. Open.
- ⚠️ Q7 (worker trust model) is its own problem. Open.
- ⚠️ Big-app scaling is its own problem. Hooks reserved; commitment
  deferred.
- ✅ App authors who want commutative merge embed CRDT inside their
  own state-apply (D pattern, documented later).

## Sources

- `convergence-and-determinism.md` (this folder) — full options + tradeoffs.
- `prior-art/willow/` — shipped event-log replay.
- `prior-art/agoric-endo/` — production vat-replay validation.
- `prior-art/holochain/` — DHT-validation precedent + unfinished sharding.
- `prior-art/crdts/lessons.md` — don't-bake-CRDT-in-kernel consensus.
- `prior-art/croquet/` — lockstep rejected; mechanics inherited.

## Q1 — MVP demo app shape

**Decision: A. Two tiny apps coexisting (counter + poll)**

Counter app (`{ value: u64 }`, increment/decrement/reset events,
~50-100 LOC state-apply + ~100-150 LOC interaction) and poll app
(`{ options: Vec<String>, votes: Map<peer, option_index> }`,
multi-author votes, permission-gated end). Both run in same kernel,
different topics, different state-component hashes; events do not cross.

**Rationale:**

- **Hits all 6 PR #636 acceptance criteria**, including coexistence (#4)
  which single-app MVPs fail.
- **"Kernel doesn't know chat" is unambiguous.** Neither app is
  chat-shaped; substrate-genericity is proven.
- **Forces multi-tenancy decisions early** (per-app namespace, per-app
  fuel budget, per-app key handles). Those bugs are cheap when
  MVP-shaped, expensive at app #5.
- **Demo optics**: two visible apps in same kernel = compelling without
  recapitulating Willow.
- **Encryption pressure deferred.** Counter + poll don't need encrypted
  state, so MLS adoption (Q9) stays deferrable.
- **Willow migration deferrable.** MVP is genuinely independent of
  Willow's chat shape.

**Variant accepted (optional v1.1)**: counter app gets an
"auto-reset-at-midnight" *behavior component* running on a designated
peer. Tests acceptance criterion #6 (behavior profile) explicitly. Adds
~1 week.

**Runners-up + why rejected:**

- **B. Single counter only.** Smallest MVP (~3-5 weeks Extism), but
  fails coexistence (#4). Multi-tenancy bugs surface late = MVP doesn't
  actually validate runtime. Rejected; useful as *step 1* internally
  but not the whole MVP.
- **C. Real-time poll only.** Fails coexistence (#4); no advantage over
  A which doubles up for similar cost.
- **D. Single-channel non-Willow chat.** Heavier scope (~12-16 weeks);
  forces MLS to v1; recapitulates Willow. Explicitly rejected per user
  direction "we're not just rebuilding Willow."

**Code placement: β. `examples/` directory in main runtime workspace.**

Rust-idiomatic. `cargo run --example counter` works. Doubles as
reference apps for SDK users. Future split to separate repos is clean.

**Concrete shape:**

```
myrhiza/
├── Cargo.toml                  workspace root
├── crates/
│   ├── kernel/                 runtime (host)
│   ├── sdk/                    app-author surface (state-apply / propose / interaction macros)
│   ├── network/                iroh wrappers
│   ├── storage/                event log + snapshots
│   └── ...
├── examples/
│   ├── counter/                wasm32 component, depends on sdk
│   │   ├── Cargo.toml
│   │   ├── manifest.toml       capability declarations
│   │   └── src/{state,propose,interaction}.rs
│   └── poll/                   same shape
└── tests/
    ├── unit/                   per-crate
    ├── integration/            kernel-with-MemNetwork, single peer
    └── e2e/
        ├── counter.rs          one-app correctness
        ├── poll.rs
        ├── coexistence.rs      ⭐ both apps, one kernel, no event-crossing
        ├── multi_peer_convergence.rs
        └── capability_gating.rs
```

**Test tier hierarchy** (lifted from Willow): state > kernel > e2e >
browser. State tier = unit-test each app's `state-apply` directly with
crafted events (instant, no I/O). Kernel tier = kernel + MemNetwork +
apps in-process. E2E tier = real iroh transport, two peer processes.
Browser tier = post-MVP.

**Dependency direction** (load-bearing constraint): `examples/` →
`crates/sdk`. Kernel crates **never** depend on examples. Examples never
in `crates/`. If `crates/kernel/Cargo.toml` ever lists `path = "../../examples/counter"`,
that's a bug.

**Forces ratified by Q1:**

- ✅ MVP scope = two minimal apps + interaction components + optional
  behavior component
- ✅ `examples/` workspace structure committed
- ⚠️ Q2 (ABI) options narrow: Extism v1 plausible (~4-8 weeks); full
  CM v1 plausible (~10-14 weeks). Both viable for this MVP shape.
- ⚠️ Q4 (browser) — A doesn't *force* browser at v1. Native-only-v1
  + browser-v2 is acceptable.
- ✅ Multi-tenancy decisions become v1 work (per-app fuel budget,
  per-app namespace).
- ✅ MLS deferrable to v2.
- ✅ Willow migration deferrable.

## Q2 — ABI choice

**Decision: A. Full Component Model day-one**

WIT-bindgen for the SDK (Rust). Wasmtime native runtime. jco-transpiled
glue + core wasm in browser (when shipped — see Q4). WIT files describe
every interface (`state-apply`, `state-propose`, `interaction`, `behavior`,
`host:*`, `ui:*`). Apps compile to CM components; kernel hosts via
Wasmtime's component API.

**Rationale:**

- **ABI final at v1 = zero migration churn.** Every Myrhiza app written
  against v1 stays valid through v2+. Willow refactor onto Myrhiza is
  one rewrite, not two.
- **Ecosystem alignment.** wasm-tools, wkg, componentize-*, wac,
  wit-bindgen are industry-standard. App authors already know the
  tooling.
- **Resource handles** for inter-component composition. Non-forgeable
  refs, borrows, world composition. Q8 (composition) collapses to
  "typed resource handles" — pre-decided by Q2.
- **Strict typing across boundaries.** Variants, records, lists,
  options, results — all WIT-typed.
- **Production-ready today.** WASI 0.2.x stable since 2024; preview3 in
  RC since 2026-01. Wasmtime 44.0.1 audited and powering Spin +
  wasmCloud + Fastly Compute@Edge.
- **Submit-and-poll already committed**, so no async loss vs Extism.
- **Avoids wasmCloud v1→v2 pivot trap.** Don't ship something we'll
  throw away.

**Counter-argument considered + dismissed**: PR #636 leans Extism for
ship-faster reasoning. That reasoning was *Willow-flavored* (chat
product to keep alive while runtime work ships). Myrhiza-as-separate-
project doesn't have a chat-product-keep-alive pressure. Schedule
argument for Extism weakens significantly. ~10-14 week ship is
acceptable.

**Runners-up + why rejected:**

- **B. Extism v1, migrate to CM later.** Faster v1 (~4-8 weeks) but
  forces double-rewrite for every Myrhiza app, including Willow when it
  eventually refactors. Migration is real refactor (resource handles
  replace ID lookups, imported interfaces replace kernel-broker calls,
  borrows replace clone-and-pass) — not regen-bindings event. No
  deadline pressure to justify the cost.
- **C. CM-shaped Extism subset.** Worst-of-both: ship slower than B,
  migrate harder than A. No production precedent for the
  bindgen-mirror-layer; bus factor on the layer if we author it.
- **D. Native-only-no-WASM-v1.** Defeats the project. Capability-secure
  WASM apps IS the point of Myrhiza.

**Forces ratified by Q2:**

- ✅ WIT is the contract surface for every host import + every component
  interface
- ✅ Cross-component composition uses typed resource handles (Q8 closes)
- ✅ Tooling stack: wit-bindgen + wasm-tools + cargo-component + wac
- ✅ Wasmtime native + jco browser (Q4 options narrow)
- ⚠️ MVP ship time ~10-14 weeks (acceptable but plan for it)
- ⚠️ App authors learn CM toolchain (real but bounded learning curve)
- ⚠️ Component instantiation overhead (~ms range) — fine for state-apply
  replay, bites if we instantiate per-event; cache instances aggressively
- ✅ MLS deferrable (Q9) — kernel commits to `host.mls` *bindings* using
  WIT but implementation can lag

## Q4 — Browser-first vs native-first v1

**Decision: C. Dual-stack v1**

Both native (Wasmtime) and browser (jco) targets ship at MVP. Kernel
internals abstract over Wasmtime + jco backends behind a stable
internal trait. Tests run on both backends.

**Rationale:**

- **No "v1.5 slip" risk.** Things scoped as fast-follow sometimes slip
  indefinitely; ratifying both targets at v1 forecloses that.
- **Browser is the project's pitch surface** ("P2P apps in your
  browser"). Shipping native-only at v1 undersells.
- **Architecture pressure on backend abstraction is healthy.** Forces
  clean separation between Wasmtime-specific code and jco-specific code.
  Avoids painful retrofit later.
- **Willow refactor unblocked at v1.** Willow is browser-shipped today;
  having browser at Myrhiza v1 means Willow refactor can land on v1
  rather than waiting for v2.
- **Browser bugs caught early** rather than discovered piecewise after
  v1 ship.
- **Most honest framing** — "we support both from day one" matches the
  P2P-apps-in-browser project signal.

**Trade accepted**: ~16-20 weeks MVP ship vs ~10-14 weeks native-only.
Heavier engineering accepted. Compounding risk (every decision evaluated
against both backends) is real but manageable with discipline.

**Runners-up + why rejected:**

- **B. Native-first + v1.5 browser fast-follow.** Lower-risk path but
  v1.5 discipline can slip; user explicitly wants both at v1 to
  foreclose that risk.
- **A. Browser-first.** Counter-productive — native is easier to
  iterate on; browser-only-v1 means slower feedback loop on substrate
  issues.
- **D. WebView-only via Tauri/Wails.** Skips jco but loses pure-browser
  deployment story. Project pitch weakens to "P2P apps on desktop."

**Forces ratified by Q4:**

- ✅ Wasmtime backend + jco backend both at v1
- ✅ Internal trait abstraction over Wasmtime + jco (design at master-
  spec time, not retrofit)
- ✅ jco preview2 is the v1 target; preview3 when stable migrates
  in-place (no API churn for app authors)
- ✅ Test tiers at v1: state (instant) / kernel-with-MemNetwork
  (per-backend, instant) / e2e-native (real iroh) / e2e-browser (jco,
  headless)
- ✅ Multi-peer browser demo path: jco-shimmed kernel + iroh-relay-
  bridged QUIC for browser-to-native + browser-to-browser
- ⚠️ Compounding risk: every architectural decision evaluated against
  both backends
- ⚠️ ~16-20 week MVP ship time
- ✅ Willow refactor onto Myrhiza targets v1 (browser available from
  v1 ship)
- ⚠️ Async story stays sync (preview2 limit) — submit-and-poll the only
  path until preview3 stabilizes
- ⚠️ ~350KB jco shim floor accepted as cost of browser-first parity

## Q5 — Sybil-resistant participation primitive

**Decision: C (social-graph Sybil resistance) as primary direction, with
explicit commitment to module-based composability (see "Architectural
commitment: module ecosystem" below).**

Master spec commits:
1. **C — social-graph Sybil resistance via invite/permission trust
   graph** as the *primary recommended* direction for apps that have a
   membership model.
2. **The participation primitive is itself a WASM module apps depend on**,
   not a kernel-built-in. Apps pull in their preferred participation
   strategy as a dependency.
3. **Pre-packaged official modules** for common cases: social-graph
   (C), tit-for-tat (A), and other variants as warranted. Third parties
   can author additional variants.
4. **Apps are not locked to a specific pattern.** Anonymous-bulletin
   apps, financial-record apps, chat apps can each pick the shape that
   fits their threat model.

**Rationale:**

- **C is the strongest standalone direction.** Leverages Willow's
  existing permission/invite trust graph (the unique Myrhiza
  advantage); Sybil-resistant by construction; aligns with
  peers-as-infrastructure framing.
- **Module-based composability is a master-spec architectural
  commitment.** App authors should not have to re-implement Sybil
  resistance, governance, RBAC, snapshot management, or other
  cross-cutting concerns from scratch.
- **Apps choose primitives based on threat model.** Membership-based
  apps use C (social-graph); bandwidth-bound roles add A (tit-for-tat);
  high-stakes durable data apps add E-style storage proofs when
  warranted.
- **WCM world composition (Q2-A) makes this natural.** Modules ARE WASM
  components with declared imports/exports; apps compose them via wac
  or runtime composition.

**Runners-up + why rejected:**

- **F (hybrid as monolithic kernel feature).** User reframed as
  composable modules — strictly better than monolithic hybrid because
  apps that don't need a primitive don't pay for it.
- **A (tit-for-tat) as primary.** Bandwidth-only solution; doesn't
  Sybil-resist alone; apps that want it can pull as a module.
- **B (reputation aggregation).** Sybil-vulnerable alone; production
  precedents thin.
- **D (DHT-responsibility) as participation primitive.** Throws out
  Q3-A paradigm; reserved as Q3 v2+ scaling-direction layered on top of
  event-log.
- **E (storage proofs).** Heavy machinery overkill for chat-shape;
  reserved as opt-in module for high-stakes-data apps.

**Forces ratified by Q5:**

- ✅ Master spec commits C as recommended direction for membership-based
  apps
- ✅ Master spec commits the **module ecosystem pattern** (see below)
- ✅ Maintenance-as-4th-component-profile (PR #636) proceeds; the
  participation enforcement strategy itself is a *module* the
  maintenance components import
- ⚠️ Anonymous participation must use a different module (e.g. tit-for-tat
  or storage-proofs); call out explicitly in master spec
- ⚠️ Apps without invite/permission graphs use an alternate module or
  accept non-Sybil-resistant participation
- ✅ v1 ships zero participation enforcement; module ecosystem starts
  empty; first module lands when first scaling demand emerges

---

## Architectural commitment: module ecosystem

User's Q5 answer surfaced a pattern that's bigger than participation
enforcement — it's the **second-tier composition layer** between kernel
and apps. This is a master-spec-level architectural commitment that
deserves its own section.

**The three tiers**:

```
   ┌─────────────────────────────────────────────────────────┐
   │                  KERNEL (myrhiza)                       │
   │  Identity. Peer protocol. Event/DAG primitives.         │
   │  Component loader. Capability arbiter. Narrow native    │
   │  imports.                                                │
   └─────────────────────────────────────────────────────────┘
                            ▲
                            │ host imports
                            │
   ┌─────────────────────────────────────────────────────────┐
   │           MODULES (myrhiza-* WASM components)           │
   │  Cross-cutting concerns reusable across apps:           │
   │  - Participation: myrhiza-participation-{social-graph,  │
   │    tit-for-tat, storage-proofs, ...}                    │
   │  - Permission: myrhiza-permission-{rbac, governance,    │
   │    invite-chain, ...}                                   │
   │  - Crypto: myrhiza-crypto-{mls, channel-key, ...}       │
   │  - State: myrhiza-state-{snapshot-cache, log-prune,     │
   │    crdt-{automerge,yjs,loro}, ...}                      │
   │  - UI: myrhiza-ui-{components, theme-tokens, ...}       │
   └─────────────────────────────────────────────────────────┘
                            ▲
                            │ component imports / wac composition
                            │
   ┌─────────────────────────────────────────────────────────┐
   │                  APPS                                   │
   │  counter, poll, chat, kanban, wiki, etc.                │
   │  Composes modules + adds app-specific state-apply       │
   │  + interaction + behavior.                              │
   └─────────────────────────────────────────────────────────┘
```

**Module properties:**

- **WASM components** with their own WIT contracts. Apps declare module
  deps in their `manifest.toml`.
- **Distributed via iroh-blobs hash-pinned** (same channel as apps).
- **Versioned with semver.** Apps pin versions; SemVer-compatible
  upgrades happen without manifest changes.
- **Composed via wac** at build time OR via runtime composition through
  the kernel's component instantiation pathway.
- **Capability-declared.** Modules' capability requirements bubble up to
  apps' manifests; users see the union of declared capabilities at
  install time.

**Module categories envisioned (roadmap, not v1):**

| Category | Examples | When |
|---|---|---|
| Participation | social-graph, tit-for-tat, storage-proofs-light | When first scaling demand emerges |
| Permission | rbac, governance (proposal+vote from Willow), invite-chain | Earliest after MVP — common app need |
| Crypto | mls, channel-key, x25519-seal | When first encrypted-state app lands |
| State helpers | snapshot-cache, log-prune, crdt-{automerge,yjs,loro} | When first big-state app emerges |
| UI tokens | components, theme-tokens, accessibility-helpers | When first non-Leptos UI app lands |

**v1 scope**: zero modules ship. Master spec commits the *pattern*; the
SDK exposes the dep-pulling mechanism; module ecosystem grows
organically as needs emerge.

**Trade accepted**: master spec has more surface area (modules tier as
first-class architectural concept) but app authors and module authors
both benefit — apps don't reinvent cross-cutting concerns; module
authors have a stable distribution target.

**Forces ratified by module-ecosystem commitment:**

- ✅ Master spec has dedicated "Module ecosystem" section
- ✅ App `manifest.toml` declares module deps with version pins
- ✅ Module distribution is same as app distribution (iroh-blobs)
- ✅ Module capability requirements bubble up to apps' install-time
  capability prompts
- ✅ wac-style composition is supported (build-time) AND runtime
  composition through kernel (per PR #636 — kernel is the cap arbiter)
- ⚠️ Module versioning + semver discipline = its own design surface
- ⚠️ Bus-factor on official `myrhiza-*` modules — if we author them, we
  maintain them
- ✅ Third-party module authors get a stable target
- ✅ Q9 (MLS) gets simpler: MLS is a module, not kernel-baked
- ✅ Q10 (distribution + signing) covers both apps and modules with one
  mechanism
- ⚠️ Q11 (cap gating) needs to handle module-mediated calls (does the
  module's manifest gate, or the calling app's, or both?)

## Q7 — Worker trust model + work allocation

**Decision: No "worker" as a peer-class. All peers can perform work
via maintenance modules. Master spec covers broadly; v1 ships no
implementation; future research direction explicit.**

The framing collapses: there is no architecturally-distinct "worker
peer." Every peer participating in an app can perform maintenance
work for that app — that's just what participation means. Some peers
choose to do more (operator-deployed infrastructure); some choose to
do less (mobile clients on metered connections); some do the default
amount automatically.

**Master-spec commitments:**

1. **All peers can perform maintenance work** via maintenance modules
   (PR #636's 4th component profile). Maintenance modules are WASM
   components like any other — sandboxed, capability-gated, signed,
   distributed via iroh-blobs.
2. **Default client behavior**: peers participating in an app
   automatically instantiate cheap maintenance modules for that app
   (sync provider, replay buffer if scoped small). Expensive modules
   (full archival persister, dedicated relay) gated by per-app user UI
   ("how much do you want to contribute?").
3. **Operator-deployed infrastructure remains a valid pattern.** An
   operator can run a peer configured with all maintenance modules
   instantiated. This peer is not architecturally distinct from a user
   peer — it's just a peer that opted in to more modules. It must be
   invited into the social graph of any app it serves (Q5-C
   integration); without invitation, the participation primitive may
   refuse to route work to it.
4. **Willow's pattern (specialized native-Rust workers — relay /
   replay / storage)** is one valid deployment shape but not the only
   one. Master spec preserves this path AND opens others.
5. **Participation primitive (Q5-C social-graph) governs work
   allocation under load.** Apps with invite/permission graphs route
   maintenance work to trusted peers; apps without may use other
   modules (tit-for-tat, etc.).
6. **Scaling with participants is the explicit ambition.** As more
   peers participate, total maintenance capacity grows. No
   infrastructure deploy required.

**v1 scope**: master spec defines the framework broadly. No maintenance
modules ship at v1. Detailed implementation (resource limits, fuel
scheduling, fair-share, deny-list mechanics, default-instantiation
heuristics) deferred to v2+ child specs and explicit research.

**Future research direction (named in master spec, not committed):**
- What maintenance modules ship as official `myrhiza-*` modules?
- What default-instantiation heuristic governs cheap-vs-expensive
  module triage?
- How does participation primitive expose "this peer is willing/able to
  host module X" to other peers?
- How does the operator-deployed-infrastructure pattern bridge with
  the social-graph participation primitive (invitation discipline,
  capability advertisement)?
- Resource limit defaults for fuel + memory budgets per maintenance
  module instance.
- Fair-share between topics on a single peer hosting many.

**Rationale:**

- **"Worker as peer-class" is the wrong abstraction.** It implies a
  distinct deployment shape; reality is "peers vary in how much they
  contribute," not "some peers are workers."
- **Architecturally honest** — peer-as-infrastructure is the framing,
  not "deploy more workers."
- **Doesn't close paths forward.** Operator-deployed boxes still work;
  user-peer-as-worker still works; bots running maintenance modules
  still work. All paths preserved.
- **Defers what we don't yet know.** Detailed allocation policy
  requires research; master spec covers shape without locking.
- **Aligns with Q5-C decision** — participation primitive governs
  allocation; master spec just defines the shape, not the policy.

**Forces ratified by Q7:**

- ✅ Master spec has "Maintenance and participation" section covering
  (1)-(6) above without deeper implementation commitment
- ✅ No "worker peer" class anywhere in the kernel architecture
- ✅ Maintenance modules use module ecosystem distribution + signing +
  capability gating (same channel as apps)
- ✅ Default client (master spec defines a `myrhiza-default-client`
  reference) auto-instantiates cheap maintenance modules per app the
  user joins
- ⚠️ Detailed allocation policy + resource limit defaults require
  child specs + research before implementation
- ⚠️ Operator-deployed infrastructure pattern needs explicit
  invitation flow into apps' social graphs — call out in Q5-C details
- ✅ Master spec explicitly preserves multiple deployment patterns
  (no-deploy / partial-deploy / full-deploy) — none ruled out
- ✅ v1 ships zero maintenance modules; the framework is named, not
  implemented

## Q6 — Multi-device + behavior identity unification

**Decision: A. Unified `IdentityScope` primitive at kernel level.**

```wit
record identity-scope {
  long-term: identity-handle,           // user / bot owner / MLS member
  instance: option<instance-binding>,   // none = the long-term itself
}

record instance-binding {
  peer: peer-handle,        // which peer this instance lives on
  kind: instance-kind,      // device / behavior / mls-leaf / ...
  name: string,             // app-chosen, e.g. "laptop", "discord-bridge", "epoch-42"
}

variant instance-kind { device, behavior, mls-leaf, custom(string) }
```

Kernel custodies private keys; signs on behalf of scopes. Apps see
opaque scope handles + `host.sign_via_scope` host import. Multi-device,
behavior identity, and MLS LeafNode signing keys all flow through this
single primitive.

**Rationale:**

- **PR #636 already names the structural similarity** between
  multi-device and behavior identity. A formalizes this; B/C contradict.
- **One mechanism instead of three.** App authors learn one API.
- **MLS integration is natural.** MLS LeafNode signing keys are
  `instance-kind: mls-leaf` scopes.
- **Module ecosystem composes naturally.** `myrhiza-identity-multi-device`
  module composes the primitive into device-add/revoke flow.
  `myrhiza-identity-behavior` for bots. `myrhiza-crypto-mls` for MLS.
- **Kernel placement is non-negotiable.** Private keys never enter
  component memory (CLAUDE.md locked).
- **v1 implementation cost: zero.** Primitive shape locked in master
  spec; first impl when first device-add/revoke or behavior-bot or MLS
  app lands.

**Runners-up + why rejected:**

- **B. Separate mechanisms per domain.** Triple the design + impl work;
  contradicts PR #636 structural-similarity insight; fragments kernel
  surface.
- **C. Kernel-built-in multi-device + behavior-app-defined.**
  Asymmetric and confusing; behavior identity has security implications
  that warrant kernel-level treatment too.
- **D. Defer all three to v2.** Forces retrofit; PR #636 already
  commits behavior identity as kernel-level — backsliding undoes that.

**Forces ratified by Q6:**

- ✅ Master spec has "Identity primitive" section with `IdentityScope`
  shape + `instance-kind` variant
- ✅ Q9 (MLS) gets cleaner: MLS module composes IdentityScope
- ✅ Module ecosystem has identity modules as a category
- ⚠️ Detailed device-add/revoke flow is a v2+ child spec
- ⚠️ Cross-peer behavior continuity (stable bot identity across peers)
  is app-level via in-band registration events (per PR #636)
- ⚠️ Recovery semantics (lost device) explicitly deferred to
  multi-device child spec
- ✅ Kernel keys stay non-extractable; apps see only scope handles
- ✅ v1 implementation effort: zero

---

## Architectural commitment: WASM execution on every backend

User clarification surfaced an important master-spec commitment:
**apps run as WASM components on every backend, including native.**
"Native" means the kernel is a native Rust binary; it does not mean
apps run as trusted native code.

| Backend | Kernel | App execution |
|---|---|---|
| Native | Rust binary (x86_64 / ARM64) | WASM components inside Wasmtime |
| Browser | jco-shimmed JS+wasm | WASM components inside jco's wasm runtime |

**Apps see capability-gated host imports either way.** Sandbox is
mandatory regardless of kernel backend. Compiling apps to native for
performance gains is **explicitly rejected** — the only way to guarantee
"WASM code can never access more than what it's granted" is to run
everything through the WASM execution environment.

**Performance trade accepted**: ~2-5% Wasmtime overhead vs native code.
Sandbox is non-negotiable; this overhead is the cost of the security
model.

**This is critical for plugins / add-ons / third-party extensions** —
the use case the user named explicitly. Apps and modules from untrusted
authors must be sandboxed; running them as native code (even with
capabilities) gives no real isolation.

**Implications:**
- Q2-D (native-only-no-WASM-v1) rejection holds — both for v1 and
  permanently.
- Cross-platform native binaries (macOS / Linux / Windows / iOS /
  Android) all use Wasmtime as the app runtime layer.
- Mobile WASM story uses Wasmtime's `cranelift` JIT or `winch` baseline
  compiler depending on platform AOT constraints (iOS prohibits JIT —
  requires AOT-only path, which Wasmtime supports).
- WebView-based desktop wrappers (Tauri / Wails) use the native kernel
  binary, not the browser kernel — keeps native iroh transport.

## Q9 — MLS adoption shape

**Decision: B. Kernel ships crypto primitives; MLS lives as
`myrhiza-crypto-mls` module.**

User clarification: "we provide a built-in solution and it is optionally
available as well, but let's just do B for now." The shape: an
**official `myrhiza-crypto-mls` module** ships authored by us as the
canonical recommended choice, but it's a *module* — app authors opt to
pull the dep or replace with an alternative.

Master spec commits:

1. **Primitive crypto host-import surface** at v1 master spec. WIT
   contract for: `host.sign_via_scope`, `host.verify_sig`,
   `host.x25519_ecdh`, `host.hkdf_derive`, `host.aead_seal`,
   `host.aead_open`, `host.hash`. All secrets bound to opaque key
   handles (via IdentityScope from Q6) — never plaintext exposed to
   components.
2. **`myrhiza-crypto-mls` as the canonical/official MLS module** when
   first MLS-needing app emerges (v2+). RFC 9420 implementation in WASM
   over the primitive surface. Kernel-custody preserved.
3. **Other crypto modules compose the same primitives**: channel-key,
   double-ratchet, sealed-content variants.
4. **Cremers ETK 2025 constraint locked**: IdentityScope long_term
   identity uses Ed25519 (SUF-CMA), not ECDSA. Documented explicitly to
   prevent regression.

**Rationale:**

- **Aligns with module ecosystem (Q5).** MLS is cross-cutting; module
  is the right placement.
- **Avoids vendor lock-in.** Kernel doesn't bake OpenMLS or any specific
  impl. Module authors compete; users choose.
- **Post-quantum migration = swap module, no kernel break.**
- **Kernel surface stays minimal.** Primitive crypto is small + serves
  many modules.
- **Future flexibility.** User flagged "not quite sure a built-in is the
  right choice yet" — module path keeps the option open. If we later
  decide kernel-baked MLS is right, that's an additive ABI change
  (`host.mls.*` host imports added later); current decision doesn't
  close that path.

**Performance trade**: WASM-MLS group operations ~2-5x slower than native
MLS. Group operations (epoch update, commit) are not hot-path so
acceptable. Application encryption (per-message) bypasses the DAG per
Willow seal-gift-wrap deferral spec, so MLS protocol cost lands only on
group state changes.

**Runners-up + why rejected:**

- **A. Kernel ships full MLS engine.** Vendor lock-in (OpenMLS or
  equivalent); kernel surface grows substantially; protocol churn
  (RFC errata, post-quantum) drags kernel; contradicts module-ecosystem
  framing.
- **C. Hybrid (kernel custodies MLS-specific state, module implements
  protocol).** Adds MLS-specific kernel surface for marginal gain over
  B; secret-handle approach in B already covers what C wants.
- **D. Defer entirely.** Forces v2 retrofit of crypto primitives, which
  are needed regardless (channel-key, sealed-content).

**Forces ratified by Q9:**

- ✅ Master spec has "Crypto primitives" section with primitive WIT
- ✅ Master spec has "MLS as module" subsection deferring
  `myrhiza-crypto-mls` to v2+
- ✅ IdentityScope (Q6) integrates as the key-handle origin for crypto
  primitives
- ✅ Cremers ETK 2025 constraint documented
- ✅ Other crypto modules (channel-key, sealed-content) get stable
  primitives
- ✅ v1 ships primitive crypto host imports; first MLS module ships
  when first MLS-needing app emerges
- ⚠️ Primitive crypto WIT shape is its own design surface (child spec)
- ⚠️ Performance ~2-5x slower than native MLS for group ops
  (acceptable)
- ⚠️ Kernel-baked MLS path stays open as future-additive ABI change if
  module approach proves insufficient

## Q11 — Capability gating mechanism

**Decision: C. Layered gating across three boundaries.**

1. **App boundary**: app's manifest declares ambient capability set
2. **Module boundary**: app and module manifests intersected at link
   time. Module's effective scope = $A \cap M$. Module can never exceed
   app's grant; app can't grant module more than module declared
   needing.
3. **Per-call gating**: high-value ops (clipboard, file picker, push,
   navigation, AEAD seal/open with sensitive keys) re-check the
   *calling component's* manifest at every call.

Plus **CM resource handles** (free from Q2-A) for non-forgeable refs
between components. Apps pass scoped handles to modules for explicit
capability transfer (e.g. "module M may write to *this* private channel
only").

**Rationale:**

- **Defense in depth.** Each layer catches a different attack class:
  manifest intersection catches malicious module declaring more than
  needed; per-call gating catches social-engineering at runtime;
  resource handles catch capability forgery; ambient set bounds
  blast radius.
- **Module ecosystem demands containment.** Pulled-in modules (third-
  party or our own) must be containable; A (per-call only) gives
  modules ambient app-level authority — too permissive.
- **Resource handles come free with Q2-A** (full CM); using them for
  capability passing is natural.
- **PR #636's per-call gating preserved as one layer**, not the whole
  story.

**Runners-up + why rejected:**

- **A. Per-call gating only** (PR #636 default). Modules get ambient
  app-level authority; no defense if module compromised. Single layer.
- **B. Full ocap discipline (Spritely E-style).** Authoring cost too
  high; app authors won't learn it. C gets most of B's benefits via
  resource handles + manifest intersection without the friction.

**Forces ratified by Q11:**

- ✅ Master spec has "Capability gating" section with three layers + CM
  resource handles
- ✅ Manifest schema (v1) supports capability declarations + module
  dep capability requirements
- ✅ Per-call gating WIT annotation for high-value ops
- ✅ Module capability scope = intersection with app
- ✅ Resource handles between components
- ⚠️ "High-value op" list at v1 is its own design surface (which kernel
  host imports get per-call gate vs ambient within manifest scope)
- ⚠️ Manifest schema is its own design surface (versioning, capability
  vocabulary, dep declaration syntax)

---

## Q10 — App distribution + signing trust root

**Decision: A. Ed25519-over-manifest+content hash, author-self-signed,
iroh-blobs distribution. P2P-native.**

**Bundle shape** (apps and modules use same shape):

```
bundle/
├── manifest.toml          author pubkey + version + capabilities + module deps + signature
├── components/
│   ├── state-apply.wasm
│   ├── state-propose.wasm
│   ├── interaction.wasm
│   └── behavior.wasm      (optional)
├── ui-assets/             (optional)
└── signature              Ed25519 over (manifest_hash + content_hash + version + author_pubkey)
```

**Install flow:**

1. User receives bundle hash via out-of-band channel (link, QR,
   in-app share).
2. Kernel fetches bundle via iroh-blobs by hash.
3. Kernel verifies Ed25519 signature against author pubkey embedded in
   manifest.
4. Kernel shows user: author identity (bech32m-encoded peer ID),
   version, capability summary.
5. User confirms; kernel instantiates.

**Author identity = IdentityScope (Q6).** Same primitive used for user
identity, bot identity, MLS member identity. App author identity is
just an IdentityScope long_term identity.

**Versioning**: semver. Bundle hash changes per version (content-
addressed). New versions are new hashes; users opt-in to upgrades.

**Rationale:**

- **P2P-native.** No central registry; matches iroh-blobs commitment
  + Q4-C dual-stack browser/native posture.
- **Lowest infrastructure cost.** No registry to operate; no Sigstore
  dependency.
- **IdentityScope reuse.** App author identity is the same primitive as
  user/bot/MLS identity — consistent surface.
- **Bech32m-encoded author identity** at install time is human-readable
  trust signal.

**Runners-up + why rejected:**

- **B. OCI artifacts + cosign/sigstore.** Centralizes what we made P2P;
  sigstore Public Good Instance is single point of failure;
  registry-flavored — wrong ecosystem alignment.
- **C. Hybrid (iroh-blobs + sigstore bundle format).** Adds complexity
  without P2P benefit; centralization sneaks back in via Rekor.
- **D. No signing in v1.** Phishing-shape attacks trivial; users have
  no author identity guarantee at install time; forces v2 retrofit.

**Forces ratified by Q10:**

- ✅ Master spec has "Distribution + signing" section with bundle
  shape + install flow
- ✅ Apps + modules use same distribution mechanism
- ✅ IdentityScope reuse for author identity
- ✅ bech32m-encoded author identity in install UI
- ✅ Semver versioning convention
- ✅ No central registry; no sigstore dependency
- ⚠️ Discovery is out-of-band at v1 (link/QR/in-app share); in-band
  catalog gossip = future direction (child spec)
- ⚠️ Bundle revocation (author retracts bad version) = child spec
- ⚠️ Author key compromise risk; mitigated by user-visible identity at
  install + revocation pattern in child spec
- ⚠️ Manifest TOML schema is shared with Q11 (capability declarations,
  module deps); coordinate child specs

## Q12 — Float discipline in state-apply

**Decision: A. Ban floats from state-apply at v1.** Escape hatch
documented for future relaxation.

App authors use scaled integers for any numeric state in state-apply
(i64 millis for time, fixed-point for positions, integer counts for
tallies). Cross-platform NaN canonicalization, SIMD float divergence,
FMA contraction — all moot.

**Profiles unaffected**: state-propose, interaction, behavior may use
floats freely. Only state-apply has the constraint.

**Implementation**: kernel rejects state-apply WASM modules that import
or use float ops at component-install time. Lint-shaped check.

**Future relaxation**: apps that genuinely need floats can opt into
B-mode via manifest declaration `state-apply.allow-floats = true`
(deferred to a future child spec; v1 default = banned). Future change
is additive — never breaking.

**Rationale:**

- **Zero divergence risk.** Banning is the simplest correctness
  argument.
- **Counter+poll don't need floats.** MVP works under ban.
- **Most state-apply code doesn't need floats.** Counts, IDs, hashes,
  timestamps are all integer-shaped.
- **Debugging cross-peer divergence from float behavior is painful.**
  Avoid the problem entirely at v1.

**Runners-up + why rejected:**

- **B. Allow spec-pinned floats with NaN canonicalization + SIMD ban.**
  Engineering surface for canonicalization + lint; debugging risk;
  reserved as v2 escape hatch.
- **C. Allow with audit.** Divergence surfaces in production as state
  mismatch — load-bearing bug; users blame Myrhiza for app author
  errors.

**Forces ratified by Q12:**

- ✅ Master spec has "Determinism: float discipline" subsection
- ✅ v1 banned in state-apply; other profiles unaffected
- ✅ Future opt-in via manifest declaration documented
- ✅ Component-install-time lint check rejects state-apply with float
  ops

---

## Session summary — all 12 questions closed

| # | Question | Decision | One-line rationale |
|---|---|---|---|
| 1 | MVP demo app shape | A. Counter + poll coexisting; `examples/` workspace | All 6 acceptance criteria; not chat-shaped; multi-tenancy tested early |
| 2 | ABI choice | A. Full Component Model day-one | Zero migration churn; ecosystem alignment; submit-and-poll commitment carries |
| 3 | Convergence paradigm | A. Event-log replay | Production-validated (Agoric, Willow); state-apply purity maps cleanly; mechanical Willow migration |
| 4 | Browser-first vs native-first | C. Dual-stack v1 | No v1.5 slip risk; browser is project pitch surface; pressure on backend abstraction |
| 5 | Sybil-resistant participation | C primary + module-ecosystem commitment | Social-graph leverages Willow's invite/permission graph (unique advantage); composable as modules |
| 6 | Multi-device + behavior identity unification | A. Unified `IdentityScope` primitive | PR #636 structural similarity formalized; MLS LeafNode keys fit; one mechanism instead of three |
| 7 | Worker trust model | No worker class — peers do work via maintenance modules | Architecturally honest; doesn't close paths; v1 ships zero, master spec covers framework broadly |
| 8 | Cross-component composition | (auto-resolved by Q2-A) Typed resource handles via CM | Free from full-CM choice |
| 9 | MLS adoption shape | B. Kernel ships crypto primitives; `myrhiza-crypto-mls` as module | Aligns with module ecosystem; no vendor lock-in; PQ migration = swap module |
| 10 | App distribution + signing | A. Ed25519 over manifest+content hash; iroh-blobs; P2P-native | No central registry; matches iroh-blobs commitment; IdentityScope reuse |
| 11 | Capability gating | C. Layered (manifest intersection + per-call gating + CM resource handles) | Defense in depth; module ecosystem demands containment; CM handles come free |
| 12 | Float discipline | A. Ban from state-apply at v1; escape hatch documented | Zero divergence risk; MVP doesn't need; future-additive opt-in |

## Cross-question architectural commitments

Three commitments emerged that are larger than any single question and
will form their own master-spec sections:

### 1. Module ecosystem (three-tier architecture)

**Kernel ↔ Modules ↔ Apps.** Modules are reusable WASM components
encapsulating cross-cutting concerns (participation primitives,
permission patterns, crypto modules, state helpers, UI tokens). Apps
declare module deps in `manifest.toml`. Modules use the same
distribution + signing + capability gating mechanism as apps. Surfaced
during Q5; ratified throughout.

**Forces**: master spec has "Module ecosystem" section. App manifests
have module-dep declarations. Module capability requirements bubble up
to app's install-time capability prompt. wac-style composition + runtime
composition both supported.

### 2. WASM execution on every backend (sandbox is non-negotiable)

Apps run as WASM components on every backend. "Native" means the kernel
is a native Rust binary; apps still execute under Wasmtime. Browser uses
jco-shimmed kernel + jco's wasm runtime. Compiling apps to native code
for performance is **explicitly rejected** — capability sandbox requires
WASM execution. ~2-5% Wasmtime overhead accepted as security cost.

**Forces**: cross-platform native binaries (macOS / Linux / Windows /
iOS / Android) all use Wasmtime. iOS uses Wasmtime AOT path (no JIT).
WebView desktop wrappers use the native kernel binary, not browser
kernel.

### 3. IdentityScope as kernel primitive

Single primitive `IdentityScope { long_term, instance: Option<binding> }`
with `instance-kind` variant covering device, behavior, mls-leaf,
custom. Kernel custodies private keys; signs on behalf of scopes. App
authors, user multi-device, behavior identity, MLS LeafNode signing,
and module signing all flow through this primitive. Surfaced during Q6;
referenced from Q9, Q10.

**Forces**: master spec has "Identity primitive" section. WIT shape
defined. Detailed device-add/revoke flow + MLS integration mechanics +
recovery semantics deferred to child specs.

## Already-locked decisions (CLAUDE.md / pre-brainstorm)

Carried in unchanged from CLAUDE.md or unanimous prior-art consensus:

- Four-component-profile split (state-apply / state-propose /
  interaction / behavior)
- Strict state-apply purity (pure function over events, no clock /
  random / network / FS / threads)
- Pre-check is mechanically the same WASM function as state-apply (in
  dry-run mode); failure-closed
- Capabilities are the only host surface
- Cross-peer convergence via app-exported `state-digest()` (not raw
  WASM linear memory hash)
- Sorted-collection serialization discipline (`BTreeMap` / `BTreeSet`)
- iroh as transport (gossip + content-addressed blobs +
  dial-by-pubkey)
- Three-layer persistence (event log canonical / per-component KV /
  snapshot cache)
- Peer-symmetric kernel (no client/server distinction)
- Kernel-only private signing keys
- Apps as bundles of WASM components
- Don't bake CRDT into kernel
- Willow stays separate; refactors onto Myrhiza later

## Already-rejected paradigms (cite-and-move)

Carried in from corpus consensus:

- Lockstep deterministic VM (Croquet) as primary paradigm
- CRDT in kernel
- Pears stack specifically (Hypercore + Hyperswarm + Bare)
- Bevy as primary UI substrate
- Shared memory between components
- Transparent distributed RPC (wasmCloud v1 → v2 lesson)
- Native-only-no-WASM-v1 (Q2 option D)

## Future-direction items (named-but-deferred)

Master spec acknowledges these as open problems with direction
committed; child specs pick up implementation:

- **Scaling problem**. Event-log replay scales linearly; v2+ likely
  evolution = DHT-sharding (Q3 option C). Other paths preserved
  (cooperative pinning, log-pruning, derived-state replication).
- **Distributed maintenance + participation enforcement**. Q5-C as
  primary direction; first module lands when first scaling demand
  emerges.
- **Multi-device device-add/revoke flow**. IdentityScope shape locked;
  flow detail = child spec.
- **MLS integration mechanics**. `myrhiza-crypto-mls` module ships when
  first MLS-needing app emerges.
- **Cross-app authority composition**. Out of scope for v1.
- **Topic-ID rotation through dumb relays without leaking next-topic
  IDs publicly**. Out of scope for v1.
- **Hot-reload**. v2.
- **Bundle revocation** (author retracts bad version). Child spec.
- **Manifest schema** (capability vocabulary, module dep declaration
  syntax). Child spec.
- **High-value-op list** for per-call capability gating (Q11). Child
  spec.
- **Worker capability advertisement** (which app/module hashes a peer
  is willing to host). Operator-config at v1; in-band gossip = v2
  child spec.
- **Snapshot portability across component upgrades**. Child spec.
- **Recovery semantics** (lost device). Child spec via
  `myrhiza-identity-multi-device` module.

## Ready for master-spec authoring

All decisions captured. Cross-question commitments named. Forces +
deferred items catalogued. Next step: write the master spec at
`docs/specs/2026-05-09-myrhiza-master-design.md` using this log as
input.

