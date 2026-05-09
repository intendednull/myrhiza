**Date:** 2026-05-09
**Status:** active
**Subject:** Pears / Holepunch — Validates / avoid / borrow tables for Myrhiza spec authors

# Lessons

This is the **consult-when-designing** file. The other files in this folder
are evidence; this file is what to *do* with that evidence when writing a
Myrhiza spec.

Three sections:

1. **Validates** — patterns Pears proves are right. Borrow the *direction*
   without reservation; you may pick a different implementation.
2. **Avoid** — patterns Pears demonstrates the cost of. Do not import these.
3. **Borrow** — concrete techniques worth lifting wholesale, with the
   citation back to where Pears does it.

Each cell earns its place. If you can't articulate why a row is here,
delete it.

## 1. Validates — Patterns Pears Proves Are Right

These are the architectural choices Pears has shipped on consumer mobile
(low-tens-of-thousands MAU class via Keet — see README's honest-scale
disclosure) and which the Myrhiza specs should treat as defaults until
someone proves otherwise.

| Pattern | Why Pears proves it | Myrhiza application |
|---|---|---|
| **Append-only signed log as the state-event substrate** | Hypercore is the entire data layer of Keet; 10+ years of production use; signed-by-author keypair gives unforgeable provenance | `state-apply` should consume an append-only signed event log; events are addressed by `(author key, sequence)` |
| **Deterministic merge for cross-peer convergence** | Autobase produces the same view across replicas given the same input blocks; this is the core mechanism that makes Keet rooms converge | Myrhiza's `state-apply` must be a pure function of `(prior state, event)` — Pears validates that this is achievable in production |
| **Sparse replication** | Hypercore peers download only the blocks they need; this is what makes Keet usable on cellular networks | Replicate event ranges, not whole logs; mobile spec should assume sparse-by-default |
| **Noise-IK + ed25519 for connection security** | Used across Hyperswarm at the DHT layer (`new NoiseHandshake('IK', ...)` in `hyperdht/lib/noise-wrap.js`); survives mobile-network scrutiny; standard primitive choice | Adopt directly; don't invent a new handshake |
| **"App = data on the network" deploy shape** | `pear://<key>` resolves to a Hyperdrive containing a versioned app bundle; updates propagate via the same data layer | Adopt the "app is content-addressed, not URL-addressed" framing; this is also what wasmCloud / Iroh / Spritely converge on |
| **Stateful DHT for peer discovery** | Hyperswarm DHT with 32-byte topics works at production scale across mobile networks | Adopt DHT-shaped discovery; *but* explicitly document bootstrap operators (see Avoid) |
| **Multiplexing multiple protocols over one stream** | `protomux` runs multiple protocol channels over a single Hyperswarm-secret-stream connection; this is how Keet runs control + data + RPC over one socket | Adopt — every stream from peer A to peer B should be multiplexable, not single-purpose |
| **App-published-by-key with multisig for production releases** | `pear multisig` (added v2.5.0) requires N-of-M signatures before a key publishes a new app version | Adopt for app distribution; multisig-on-publish is the "anti-rogue-publisher" defense the substrate needs |
| **Mobile-first runtime (Bare) is achievable** | Keet shipping on iOS at 4.59★ with binaries small enough for App Store review proves the deploy shape works | Don't accept "P2P apps can't ship on phones" as a given; Pears proves they can |
| **Single-developer-keypair as default app identity** | Keet's per-device-keypair model works for chat use cases | Adopt as the *default* identity primitive; build user-level identity on top per-app |
| **Compact binary encodings over JSON for wire format** | `compact-encoding` is used throughout for size-efficient binary serialization | Adopt — JSON is the wrong default for P2P wire protocols on cellular |

## 2. Avoid — Patterns Pears Demonstrates the Cost Of

These are the choices Pears has made that we have evidence are problems.
*Do not import these into Myrhiza.* The justification column cites the
critique that supports the avoid.

| Pattern | Why avoid | Citation |
|---|---|---|
| **JS-only application substrate** | No substrate-level type system, no determinism enforcement, no language interop without FFI; locks ecosystem out of Rust/Go/Swift native components | `critiques.md` §"JS-Only Stack"; pear#202 (TypeScript open issue) |
| **Closed-source flagship application** | Outside teams can't verify the deploy shape works; edge-case behavior (iOS background, push, NAT pairs) is a black box | `critiques.md` §"Closed-Source Flagship App"; `keet-mobile-releases` is binary-only |
| **Single-vendor governance with no foundation** | Concentration risk, no neutral protocol-spec process, "any PR may be closed without explanation" | `governance.md`; `pear/CONTRIBUTING.md` verbatim |
| **"No servers" rhetoric without bootstrap-node disclosure** | Marketing diverges from technical reality; bootstrap nodes are real, operator-dependent, and Holepunch-run | `critiques.md` §"No Servers vs Bootstrap"; hyperdht/lib/constants.js `BOOTSTRAP_NODES` |
| **Major-version protocol breaks every 2-3 years without migration story** | v8 → v10 (2022) re-encode required; v10 → v11 (2025) migration required; every break is a tax on every userbase | `critiques.md` §"Hypercore Protocol Version Churn"; tag history |
| **iOS push-notification non-answer** | Keet has a private solution; the open stack has no protocol-level answer; everyone re-solves | `critiques.md` §"iOS Background Mode + Push"; no public docs |
| **Determinism as honor-system, not substrate-enforced** | App can call `Math.random()` and `Date.now()`; cross-peer divergence detected only by comparing views after the fact | `open-problems.md` §"Determinism"; Bare exposes V8 globals unfiltered |
| **Per-application identity with no protocol primitive** | Every app reinvents user-vs-device-key reconciliation; cross-app identity is impossible | `open-problems.md` §"Identity Portability"; `keet-identity-key` is per-app |
| **Default Hyperswarm always-on connections without battery toggles** | hyperswarm#47 has been open six years asking for layer-toggle controls; mobile apps end up doing custom lifecycle management | `critiques.md` §"Performance / Battery"; hyperswarm#47 |
| **Single-investor (Tether) funding concentration** | One decision-maker controls roadmap continuity; Dat-era precedent shows what happens when a single funder steps back | `governance.md` §"Tether as Funder"; Dat→Holepunch transition |
| **Sparse / non-existent protocol specs for non-JS implementers** | Re-implementing Hyperswarm in Rust/Go/Swift means reading the JS source; no RFC | `critiques.md` §"Sparse Documentation"; hyperswarm#60 open six years |
| **In-place encryption-key rotation impossible** | Rotating the encryption key effectively forks to a new core; no forward / post-compromise secrecy at the log layer | `open-problems.md` §"Encryption-Key Rotation" |

## 3. Borrow — Concrete Techniques Worth Lifting

These are the implementation patterns Pears has gotten right at the level
of detail that's worth borrowing wholesale. Each row has a citation back to
the Pears module / repo so the spec author can read the reference.

### 3.1 Transport / Discovery / Cryptography

| Technique | Where Pears does it | What to borrow |
|---|---|---|
| **Holepunching phase-1 + phase-2** | `hyperdht/lib/connect.js`, `dht-rpc` (NAT-traversal protocol with cooperative random-punch fallback) | The two-phase protocol: consistent-NAT direct punch first, then DHT-coordinated random punch as fallback. Tail behavior matters — see `_randomPunchInterval = 20s` default |
| **Three-tier discovery (DHT + mDNS + relay)** | `hyperswarm` index.js options + `hyperswarm-dht-relay` | Use all three layers; LAN gets mDNS, internet gets DHT, NAT-blocked gets relay-as-last-resort |
| **`protomux` multiplexer over framed stream** | `holepunchto/protomux` | Single Hyperswarm connection carries N protocol channels; each app feature is a channel, not a socket. Adopt the channel-per-protocol pattern |
| **`protomux-rpc` for request/response over `protomux`** | `holepunchto/protomux-rpc` | When you need RPC semantics over a P2P stream, this is the shape — request/response channels multiplexed alongside data channels |
| **Hyperswarm-secret-stream (Noise IK over framed transport)** | `holepunchto/hyperswarm-secret-stream` (Apache-2.0) | The Noise handshake details are right; lift the construction including the framing pattern |
| **Bootstrap nodes declared as policy, not hidden as defaults** | (Pears does this *poorly* — hardcoded in `hyperdht/lib/constants.js`) | **Anti-pattern correction:** Myrhiza should declare bootstrap nodes as part of the deployment manifest, not as substrate-baked-in defaults |

### 3.2 Data Model

| Technique | Where Pears does it | What to borrow |
|---|---|---|
| **Append-only log with per-block signature chain** | `holepunchto/hypercore` core wire format | The wire-level pattern: each block signed by author keypair, merkle-tree-hashed for verifiable subset replication |
| **Per-block encryption with separable encryption key from authoring key** | Hypercore `encryption: { key: ... }` option | Decouple "who can write" from "who can read" — same pattern Myrhiza needs for room-shaped state |
| **Autobase view-derivation pattern** | `holepunchto/autobase` `apply(nodes, view, host)` callback | The structure: input is a sequence of (writer, block) tuples; output is a deterministic view; the `apply` function encodes the merge semantics |
| **Indexer vs non-indexer writers** | Autobase `addWriter` `{ indexer: true }` distinguishes writers who participate in deciding the canonical order from writers who only contribute events | Useful for trust-tiered systems: not every contributor needs to be an order-decider |
| **`hyperbee` for B-tree-over-Hypercore** | `holepunchto/hyperbee` | When you need indexed key-value over a log, this is the construction; lift the pattern, not necessarily the JS |
| **Compact-encoding schemas** | `holepunchto/compact-encoding` and `compact-encoding-struct` | Binary wire schema language — type definitions in JS that produce minimal binary encodings; useful pattern even if Myrhiza uses WIT instead |

### 3.3 Application / Identity / Access Control

| Technique | Where Pears does it | What to borrow |
|---|---|---|
| **Room-key-as-access-control** | Keet rooms (per `blind-pairing-core`) — knowing the 32-byte key *is* the join capability | Default access-control primitive. Capability = secret. Combines with Sybil-as-app-problem: if you don't know the key, you can't join, no Sybil possible |
| **Encrypted-core pattern** | Hypercore `encryption.key` + Hyperswarm DHT topic derived from key hash | Discovery topic is a public hash of the room key; only key-holders can decrypt blocks. Discovery is open; reading is gated. Adopt directly |
| **Hierarchical deterministic key derivation for user identity** | `holepunchto/keet-identity-key` (BIP32-like for Keet identity) | When an app needs "one user, multiple devices", HD key derivation gives deterministic per-device keypairs from a single seed. Not at protocol level — borrow at application level |
| **Blind pairing for adding new devices** | `holepunchto/blind-pairing-core` | Pairing protocol for "I have a new device, let me join my own account" — uses short-codes + Diffie-Hellman to add a device without exposing the seed |
| **Multisig for production-release signatures** | `pear multisig` (CLI subcommand, v2.5.0+) | Coordinator collects N-of-M signatures, then a single signed manifest publishes the new app version. Adopt this pattern for any "key-published" artifact in Myrhiza |

### 3.4 Runtime / Distribution

| Technique | Where Pears does it | What to borrow |
|---|---|---|
| **`pear://` URL scheme for app addressability** | `holepunchto/pear` runtime + Hyperdrive resolution | An app is addressable by a public key, not by a domain name. The URL scheme bakes in non-DNS identity. **Caveat:** Myrhiza will likely use a different scheme (`myr://`?) but the *shape* — content-addressed app identity in a URL — is right |
| **Mobile-binary-size discipline (Bare)** | `holepunchto/bare` runtime philosophy: "Bare itself only adds a few missing pieces on top to support a wider ecosystem of modules" | Keep the runtime small, push features into userland-modules. iOS App Store rejection rates are size-correlated |
| **OTA updates over the same data substrate** | Pear apps update by pulling new versions over Hyperdrive | Don't have a separate "update channel" — apps' data layer is also their distribution layer. New version = new content the same way new messages are new content |
| **`pear seed` for keeping app artifacts available** | `pear seed <link>` runs a long-lived seeding process | Anyone can seed; popular apps end up self-distributed. Borrow: any participant can become a (re-)distributor |
| **Embeddable runtime as a library** | `pear-runtime` module (extracted from `pear`, deprecates `pear run`) | An app runtime should be embeddable — not just a CLI. Other shells (mobile launchers, web pages, OS-level integrations) need to host the runtime |

### 3.5 Process / Operations

| Technique | Where Pears does it | What to borrow |
|---|---|---|
| **Continuous-shipping cadence visible in CHANGELOG** | `pear/CHANGELOG.md` shows weekly-to-monthly releases over 2 years | Maintain a public CHANGELOG with explicit "Features / Improvements / Fixes" sections per release. Boring. Worth doing |
| **Conventional-commits-friendly release notes** | Pear changelog isn't strictly Conventional Commits but is structured | Adopt Conventional Commits + a derived CHANGELOG so users can scan changes |
| **Public binaries for every release of the flagship** | `keet-mobile-releases` ships APK / IPA artifacts | If Myrhiza ever ships its own flagship, do this; binary distribution is what makes "shipping" believable to outside readers |

## How to Use This File

When writing a Myrhiza spec that touches:

- **State / event log** → §1 *Validates*: append-only signed log; deterministic merge; sparse replication
- **Transport / discovery** → §3.1 *Borrow*: protomux, secret-stream, three-tier discovery; *Avoid*: hardcoded bootstrap nodes
- **Identity / access control** → §3.3 *Borrow*: room-key-as-capability, encrypted-core pattern, HD key derivation; *Avoid*: per-app identity reinvention
- **Substrate / runtime** → §1 *Validates*: mobile-first is achievable; *Avoid*: JS-only, determinism-as-honor-system
- **Distribution / updates** → §3.4 *Borrow*: pear-style content-addressed app URLs, multisig releases; OTA over data substrate
- **Mobile** → §3.4 *Borrow*: Bare's binary-size discipline, embeddable-runtime; *Avoid*: privatized iOS push answer

When in doubt, the rule of thumb: **borrow Pears' implementation patterns;
do not borrow Pears' substrate, governance, or marketing posture.**

## Cross-references

- [governance.md](./governance.md) — why "single-vendor" goes in *Avoid*
- [history.md](./history.md) — why protocol-version churn goes in *Avoid*
- [comparisons.md](./comparisons.md) — what other stacks teach where Pears falls short
- [critiques.md](./critiques.md) — evidence for the *Avoid* column
- [open-problems.md](./open-problems.md) — what Myrhiza inherits if borrowing
- [pear-runtime.md](./pear-runtime.md), [bare-runtime.md](./bare-runtime.md), [hypercore-stack.md](./hypercore-stack.md) — module-level deep dives
- Prior-art neighbors: [Iroh](../iroh/), [Holochain](../holochain/), [WASM CM](../wasm-component-model/), [wasmCloud](../wasmcloud/), [Spritely OCapN](../spritely-ocapn/), [Agoric](../agoric-endo/)
