# Myrhiza

A peer-to-peer application runtime. A small kernel hosts typed, capability-mediated,
content-addressed apps built as bundles of WebAssembly components.

**Status:** pre-runtime. The master design spec is `[draft]`; foundation crates
(kernel, manifest, backend, network, types, test-utils) are in place and the
plan B-1 event DAG + cross-peer convergence acceptance has just landed on `main`.
No runnable end-user binary yet.

**License:** [AGPL-3.0-only](#license).

---

## What Myrhiza is

The kernel owns identity, peer protocol, the event/DAG primitives, the component
loader, and the capability arbiter. Everything else — chat, wikis, kanban, polls,
whatever someone builds in two years — is an app. Apps cannot touch private keys,
the network, persistent storage, or other apps' state directly: every operation
is mediated by a kernel-arbitrated capability.

Two load-bearing commitments shape the rest of the design:

- **Peers are infrastructure.** Storage, replication, sync, replay buffering,
  and snapshot custody are work performed by participants, not by deployed
  services. As more peers join an app, its maintenance capacity grows. No
  infrastructure deploy is required to scale.
- **Capabilities are the only host surface.** WASM execution is non-negotiable
  on every backend (native, browser, mobile). Compiling apps to native code for
  performance is explicitly rejected — sandboxing is the point.

### What Myrhiza is not

- Not a chat client. Chat is one app among many.
- Not a plugin framework for a host application. Apps are the product; the
  kernel is the substrate.
- Not a CRDT library. Apps may use CRDTs internally; the kernel stays generic.
- Not a service to deploy. Peers are the runtime.

The novel combination is: WASM-Component-Model-typed components + capability-secure
host surface + no CRDT in kernel + author-bounded scale at v1 + event-log-replay
convergence with TUTTI-shaped drift detection. The individual pieces are borrowed
from Holochain, Agoric/Endo, Spritely OCapN, Croquet, Pears, and the in-house
ancestor Willow. See [`docs/prior-art/`](docs/prior-art/) for the receipts.

---

## Architecture at a glance

### Three tiers

```
   ┌──────────────────────────────────────────────────────────┐
   │                       KERNEL                             │
   │  Identity. Peer protocol. Event/DAG primitives.          │
   │  Component loader. Capability arbiter. Crypto            │
   │  primitives. Narrow native imports.                      │
   └──────────────────────────────────────────────────────────┘
                              ▲
                              │ host imports (WIT-typed)
                              │
   ┌──────────────────────────────────────────────────────────┐
   │              MODULES  (myrhiza-* WASM components)        │
   │  Cross-cutting concerns reusable across apps:            │
   │  participation, permission, crypto (MLS, channel-key),   │
   │  state helpers (snapshot-cache, CRDT adapters),          │
   │  identity, UI primitives.                                │
   └──────────────────────────────────────────────────────────┘
                              ▲
                              │ component imports / wac composition
                              │
   ┌──────────────────────────────────────────────────────────┐
   │                         APPS                             │
   │  counter, poll, chat, kanban, wiki, etc.                 │
   │  Compose modules + add app-specific state-apply,         │
   │  state-propose, interaction, and behavior components.    │
   └──────────────────────────────────────────────────────────┘
```

Kernel is privileged; modules and apps are sandboxed. Modules and apps are
mechanically the same shape (WASM components with manifest + signature) — the
distinction is intent. Identity custody, capability arbitration, deterministic
replay, and network plumbing have to live in the kernel because they need
privileged native resources; everything else stays out.

### Four component profiles

Components declare a runtime profile. Profiles differ in determinism rules and
permitted host imports.

| Profile | Purpose | Determinism | Where it runs |
|---|---|---|---|
| `state-apply` | Materialize event into state; authority verdict | **Strict** — pure function of `(prior state, event)` plus the deterministic helper set | Every peer materializing the topic |
| `state-propose` | Build a candidate event from intent | Loose — kernel re-checks via `state-apply` in dry-run | The peer originating the event |
| `interaction` | UI / user-facing surface | Non-deterministic OK; per-peer | Any peer with a UI host |
| `behavior` | Bots, bridges, automations | Non-deterministic OK; per-`(peer, instance)` identity | Designated peer(s) |

Pre-check and apply are mechanically the same WASM function, called by the
kernel in dry-run vs. real-apply mode. Cross-peer convergence is proved by
construction: two peers that run the same `state-apply` against the same event
log get the same state hash. Floats are banned at v1; fuel is instruction-count
based; clocks, randomness, network, filesystem, and threads are denied to
`state-apply`.

Full host-import-by-profile matrix lives in
[`docs/specs/2026-05-09-myrhiza-master-design/architecture.md`](docs/specs/2026-05-09-myrhiza-master-design/architecture.md) §3.5.

---

## Repository layout

```
myrhiza/
├── Cargo.toml                  workspace root (resolver = "2", edition 2024)
├── Justfile                    fmt / lint / test / fixtures / spec-coverage / ci
├── rust-toolchain.toml         pinned to 1.95.0
├── crates/
│   ├── types/                  shared core types
│   ├── manifest/               manifest schema, canonical encoding, Ed25519 verify
│   ├── backend/                backend trait abstraction (Wasmtime + jco satisfy)
│   ├── wasmtime-backend/       Wasmtime impl: capability-gated linker, fuel, float-ban
│   ├── kernel/                 install flow + state-apply ABI + state-digest
│   ├── network/                Network trait + in-process MemNetwork double
│   └── test-utils/             dev-only fixtures and doubles
├── wit/myrhiza-kernel/         WIT contracts (frozen + name-resolved)
├── tests/
│   ├── fixtures/               wasm component fixtures (built via wasm-tools)
│   ├── snapshots/              golden artifacts
│   └── spec-coverage.md        spec-heading ↔ test mapping (CI-validated)
├── docs/                       specs, plans, reports, prior-art (see docs/README.md)
└── scripts/                    CI helpers
```

The workspace deliberately keeps each crate small and single-purpose. New crates
land when their first non-trivial code does, not before.

---

## Build and test

Requires Rust 1.95 (pinned via `rust-toolchain.toml`) and, for the WASM fixtures,
the `wasm32-unknown-unknown` target plus [`wasm-tools`](https://github.com/bytecodealliance/wasm-tools).

```bash
just              # default = ci: fmt-check + lint + test + spec-coverage-check
just fmt          # cargo fmt --all
just lint         # cargo clippy --workspace --all-targets -- -D warnings
just test         # cargo test --workspace --all-targets
just check        # cargo check --workspace --all-targets
just build-fixtures   # compile + wrap the 5 wasm component fixtures
just spec-coverage    # regenerate tests/spec-coverage.md
```

Raw `cargo` equivalents work too; the `just` recipes just bake in the right flags.
Zero clippy warnings is enforced; `unsafe_code = "forbid"` workspace-wide;
`panic` / `unwrap_used` / `expect_used` are `warn`-level on the state-apply
runtime path with a documented opt-out for test crates.

Fixtures are built for `wasm32-unknown-unknown` (not `wasm32-wasip2`) so they
import only the kernel's deterministic helper set — the WASI shim would pull in
imports the kernel does not provide.

---

## Documentation

- **[CLAUDE.md](CLAUDE.md)** — development guide, conventions, profile rules,
  prior-art protocol.
- **[docs/README.md](docs/README.md)** — master catalog of specs, plans,
  reports, prior-art, and reference indices, with status tags.
- **[docs/specs/2026-05-09-myrhiza-master-design/](docs/specs/2026-05-09-myrhiza-master-design/)** — the runtime master spec.
  Start with its `README.md` for the reading order. Child files cover
  architecture, convergence, determinism, identity, capabilities, ABI,
  crypto, distribution, networking, maintenance, UI, browser-native
  dual-stack, MVP acceptance, migration, future direction, tradeoffs,
  risks, implementation outline, and verification.
- **[docs/prior-art/](docs/prior-art/)** — researched deep dives on Holochain,
  Pears, Spritely/OCapN, Agoric/Endo, Willow, Iroh, WASM Component Model,
  wasmCloud, Spin, Croquet, MLS, and the CRDT library landscape.

When you touch a spec, plan, or any non-trivial change: read the master spec
README first. The prior-art folders are launchpads for further research, not
final answers.

---

## Design choices (the short version)

- **Strict-purity state-apply.** Determinism is a load-bearing property. Any
  non-determinism in `state-apply` is a correctness bug, not a quirk. Pre-check
  is mechanically the same function.
- **Capabilities are the only host surface.** Adding a host import is an ABI
  change. The kernel's deterministic helper set is the minimal trusted ground.
- **WASM Component Model + WIT.** Wasmtime LTS 36.0.9 on native; jco-shimmed
  in the browser. Components compose via `wac`. WIT contracts are frozen and
  CI-checked.
- **Content-addressed bundles, Ed25519-signed manifests.** Bundle hash is
  identity; the kernel binary is the trust root.
- **Event-log replay convergence (not in-kernel CRDTs).** The kernel arbitrates
  ordering and applies events through `state-apply`; merge semantics live in
  app code. Apps may pull in CRDT modules as dependencies.
- **iroh transport.** Dial-by-pubkey QUIC, content-addressed blob distribution,
  NAT traversal via relays. (B-4 wiring is upcoming; current `network` crate is
  the trait + in-process double.)
- **Dual-stack v1: native + browser.** Both targets are first-class. Browser
  uses jco transpile + submit-and-poll for async surfaces.
- **No worker class.** Maintenance is a deployment posture, not a runtime
  profile. Peers self-select into hosting maintenance modules.
- **AGPL-3.0-only.** Strong copyleft on the runtime.

Tradeoffs and runners-up that were rejected are documented inline in the spec
— see `tradeoffs.md` for the matrix.

---

## Future direction

Master spec commits direction on these so v1 does not paint corners.
Implementation lands in child specs when demand emerges; see
[`docs/specs/2026-05-09-myrhiza-master-design/future.md`](docs/specs/2026-05-09-myrhiza-master-design/future.md)
for the full inventory.

- **Scaling.** Event-log replay scales linearly. Likely v2+ evolution: DHT-shape
  sharding layered on top; cooperative pinning, log-pruning, and derived-state
  replication are preserved as alternative paths. Decision criteria: measure the
  bottleneck before committing.
- **Distributed maintenance.** Default-instantiation heuristics, capability
  advertisement, fair-share scheduling between topics, bridges between
  operator-deployed infrastructure and social-graph invitation discipline.
- **Identity.** Multi-device device-add/revoke, recovery for a lost device,
  cross-peer behavior continuity, and a quantum-safe signature migration.
- **Crypto.** A `myrhiza-crypto-mls` module when the first MLS-needing app
  arrives (Cremers ETK 2025: use Ed25519, never ECDSA). Channel-key, double-
  ratchet, sealed-content modules to follow. Quantum-safe primitives.
- **Capability model.** A high-value-op list for per-call gating, capability
  vocabulary in the manifest schema, cross-app authority composition (out of
  scope at v1).
- **Distribution.** Bundle revocation, in-band catalog gossip for app/module
  discovery, supply-chain hardening tooling.
- **Networking.** Topic-ID rotation through dumb relays, an `HistorySyncComplete`
  EOSE-style backfill signal, Negentropy-shape range reconciliation for very
  large topics.
- **Determinism.** Opt-in floats via `state-apply.allow-floats = true` in a
  manifest, snapshot portability across component-version upgrades, additional
  state-digest formats opt-in.
- **Interaction.** Full `ui:*` WIT contract details, custom-pixel surface
  escape hatch on non-web platforms. Hot-reload deferred to v2.
- **Module ecosystem.** Versioning + semver discipline, bus-factor handling on
  official `myrhiza-*` modules, audit / curation policy.

The MVP acceptance is two coexisting apps (counter + poll) on the same kernel,
with cross-peer convergence verified via `state-digest`, capability gating
proven, and per-app namespacing intact. See
[`docs/specs/2026-05-09-myrhiza-master-design/mvp.md`](docs/specs/2026-05-09-myrhiza-master-design/mvp.md)
§15.1 for the full acceptance criteria.

---

## Contributing

Read [CLAUDE.md](CLAUDE.md) first — it covers development principles
(quality + longevity over speed, no hacky workarounds, root-cause every bug,
verify-before-claiming-done), the spec/plan workflow under `docs/`, and the
prior-art protocol.

PR hygiene:

- [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`,
  `fix:`, `docs:`, `refactor:`, `chore:`).
- One concern per PR. If the description needs three sections, it is two PRs.
- Never `--no-verify`. If a hook fails, fix the root cause.
- Never force-push `main`.
- `just ci` must be green before review.

When a spec consults prior art, cite the folder and section
(e.g. `prior-art/mls/lessons.md §3`), name the runner-up paradigm if a choice
was made, and flag remaining gaps.

---

## License

Licensed under the [GNU Affero General Public License v3.0 only](https://www.gnu.org/licenses/agpl-3.0.html)
(`AGPL-3.0-only`). See workspace `Cargo.toml` for the declaration.
