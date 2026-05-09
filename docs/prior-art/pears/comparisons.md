**Date:** 2026-05-09
**Status:** active
**Subject:** Pears / Holepunch — How the stack compares to the other prior-art entries

# Comparisons

Six adjacent stacks. Each comparison is two tables — *transport / data* and
*deploy shape / governance* — on the dimensions Myrhiza specs care about.
Read this when you have already decided to borrow a Pears pattern and want to
double-check whether one of the alternatives is a better fit.

The dimensions that matter for Myrhiza:

- **Transport substrate** — what punches NATs, what re-keys connections.
- **State substrate** — how state propagates, what guarantees on order /
  determinism.
- **Runtime substrate** — JS / WASM / native, mobile-shippability.
- **Deploy shape** — what is "an app" addressable as, on what kind of name.
- **Determinism** — does the substrate enforce or merely allow it?
- **Governance** — single-vendor / foundation / consortium.
- **Mobile production evidence** — does the stack actually ship at scale?

## vs Iroh — the closest peer comparison

Iroh ([prior-art/iroh/](../iroh/)) and Pears solve nearly the same problem
with nearly opposite ergonomic choices. Iroh is younger but Rust-from-the-
start; Pears is older but JS-only.

| Dimension | Pears | Iroh |
|---|---|---|
| Transport | Hyperswarm (UDP holepunching, DHT, mDNS) | iroh-net (QUIC over UDP, DERP relays, holepunching) |
| Data layer | Hypercore (single-writer append-only log) + Autobase (multi-writer linearization) | iroh-blobs (BLAKE3-addressed) + iroh-docs (replicated key-value) |
| Runtime | JS via Bare or Node | Rust, WASM-friendly, mobile via uniffi/JNI |
| Mobile shipping | Keet (iOS + Android, Holepunch-built) | Delta Chat (iOS + Android, Iroh as transport since 2024) |
| Encryption | Noise IK over Hyperswarm-secret-stream | QUIC TLS 1.3 + Noise variants |
| Bootstrap nodes | 3 hardcoded Holepunch hosts | DERP relays, Number 0 + community-operated |

| Dimension | Pears | Iroh |
|---|---|---|
| Deploy shape | `pear://<32-byte-key>` resolves to Hyperdrive | `iroh-blobs://<blake3>` for content; ticket-based for live docs |
| Determinism enforcement | None — JS app must be careful | None — Rust app must be careful |
| Governance | Single vendor, no foundation | Single vendor (Number 0 Inc.), but explicit-neutral framing |
| Stack maturity | 10+ years for data layer; 2 years for runtime | 4 years (iroh started 2022); newer everywhere |
| Substrate constraints for embedded use | JS-only — can't drop into a Rust/Go/Swift host | Rust-native — embeddable into anything via FFI |

**The verdict for Myrhiza spec authors.** If your runtime is WASM-Component-Model
based, **Iroh is the closer match** — it's already designed to embed into
WASM hosts and Rust binaries. Pears' contribution to your reading is
*production-mobile evidence*, not transport-API blueprint. Borrow the Pears
*observed reality* (Keet shipping); borrow Iroh's *technical surface*.

## vs Holochain — the other consumer-shipping P2P stack

Holochain ([prior-art/holochain/](../holochain/)) and Pears are the two stacks
in this set that have actual mobile-shipping production deployments
(Keet vs. Volla Messages). They have very different consistency models.

| Dimension | Pears | Holochain |
|---|---|---|
| State model | Author-centric append-only log; signed by author keypair | Agent-centric source chain + DHT; per-agent ordered, globally CRDT-merged |
| Multi-writer | Autobase linearizes multi-writer cores into one view | Native — every agent writes their own chain, DHT validates |
| Conflict resolution | App-defined view function (deterministic merge given inputs) | Validation rules per zome; rejects invalid entries |
| Encryption | Per-core block-encryption keys (optional) | Source-chain entries can be private to agent or public via DHT |
| Mobile shipping | Keet (Holepunch direct) | Volla Messages on Volla Phone (third-party app on third-party hardware) |

| Dimension | Pears | Holochain |
|---|---|---|
| Substrate | JavaScript on Bare | WASM (Rust DNA bundled into hApps) on Holochain conductor |
| Deploy shape | `pear://<key>` | `hApp` bundle with DNA hash addressing |
| Determinism enforcement | None — JS at runtime | WASM sandboxing + per-agent validation rules — **closer to enforced** |
| Governance | Single vendor (Holepunch, Tether-funded) | Holo Ltd. + Holochain Foundation (multi-stakeholder leaning) |
| Production scale | ~99 iOS ratings for Keet (small) | Volla Messages ships on a niche phone OEM |
| Maturity of consumer rollout | Shipping 3+ years | Shipping but smaller surface |

**The verdict.** Holochain's substrate is a much closer match to Myrhiza's
WASM-component-on-deterministic-substrate model. Pears is a closer match for
"how do you ship a P2P JS app on a phone" — i.e., the operational layer.
Borrow Holochain's *substrate model*; borrow Pears' *mobile shipping
discipline* (binary size, suspend/resume, App Store review playbook).

## vs WASM Component Model + wasmCloud — the server-class substrate

WASM Component Model ([prior-art/wasm-component-model/](../wasm-component-model/))
and wasmCloud ([prior-art/wasmcloud/](../wasmcloud/)) are the substrate side
of Myrhiza's design space. Pears is on a different axis.

| Dimension | Pears | WASM CM + wasmCloud |
|---|---|---|
| Application target | Consumer mobile + desktop apps (chat, productivity) | Server / cloud / edge workloads |
| Substrate | JavaScript via Bare | WebAssembly with WIT-typed component boundaries |
| Type system at substrate boundary | None (JS dynamic) | WIT-enforced, language-neutral |
| Capability model | Implicit, library-level (you import a Hypercore) | Explicit, runtime-mediated (host imports declared at the boundary) |
| Deploy shape | `pear://<key>` over P2P Hyperdrive | OCI registry + lattice routing (wasmCloud) |
| Mobile shipping | Yes (Keet) | No, server-only |

| Dimension | Pears | WASM CM + wasmCloud |
|---|---|---|
| Governance | Single vendor | Bytecode Alliance (multi-vendor foundation) |
| Determinism | App's responsibility | Substrate-enforceable via host-import policy + WIT typing |
| "App = hash on network" pattern | Yes — `pear://<key>` | Yes — OCI digest |
| Upgrade story | New key → new app; multisig signing for updates | Image digest pin + lattice config |

**The verdict.** Different problem space. WASM CM is what Myrhiza's substrate
*is*; Pears is what Myrhiza's *consumer-mobile-deployment story* should learn
from. Don't borrow Pears' substrate (JS); do borrow Pears' deploy-shape
intuition ("an app is a key on the network, not a URL on a server").

## vs Spritely OCapN — capabilities vs replication

Spritely OCapN ([prior-art/spritely-ocapn/](../spritely-ocapn/)) and Pears
are both addressing "how do peers communicate", but at very different layers.

| Dimension | Pears | Spritely OCapN |
|---|---|---|
| Primary primitive | Replicated data (append-only log) | Object capabilities + cross-host promise pipelining |
| Communication model | Subscribe-to-log, pull blocks | Send-message-to-capability, get-promise-back |
| RPC | Separate concern — `protomux` + `protomux-rpc` over a Hyperswarm stream | Native — the whole protocol is RPC-shaped |
| State sync | Built-in (Hypercore replication) | Not built-in — apps build it on top of message-sends |

| Dimension | Pears | Spritely OCapN |
|---|---|---|
| Type system | None at the wire (CBOR / compact-encoding) | Capability-typed via OCapN |
| Trust model | Public-key-signed log; authority = key holder | Capability-as-unforgeable-reference |
| Governance | Single-vendor (Holepunch) | Non-profit (Spritely Institute) |
| Maturity | Production-shipping | Specification + reference implementations |

**The verdict.** Different design points. Pears' replication-first model is
better when "everyone needs the same data". OCapN's RPC-first model is
better when "I want to ask one specific party to do one specific thing".
Myrhiza will likely need both: borrow Hypercore's replication shape for
state, borrow OCapN's capability-typed RPC shape for explicit ask-and-answer
interactions.

## vs Agoric SwingSet — the deterministic-replay shape

Agoric ([prior-art/agoric-endo/](../agoric-endo/)) and Pears both have
deterministic-replay shape for state convergence — but on very different
substrates.

| Dimension | Pears | Agoric SwingSet |
|---|---|---|
| Determinism mechanism | Hypercore append-only log + deterministic merge function (Autobase view) | xsnap (deterministic JS) + SwingSet vat orchestration |
| What enforces determinism | App-author discipline (JS isn't deterministic by default) | xsnap engine itself — deterministic JS by construction |
| Replay model | Re-derive view from log | Re-execute vats from input log |
| Scale of state | Per-app append-only log | Per-vat heap + cross-vat message log |

| Dimension | Pears | Agoric SwingSet |
|---|---|---|
| Substrate | Bare (V8-based JS, non-deterministic) | xsnap (Moddable's deterministic JS engine) |
| Use case | Chat, social, file sync | Smart-contract / financial-state machines |
| Governance | Single vendor | Agoric the company; production-deployed Cosmos chain |
| Borrowable insight | The append-only log shape; the view-derivation pattern | Take determinism *out* of the app's hands and put it in the substrate |

**The verdict.** Agoric is the right reference if Myrhiza wants substrate-
level determinism enforcement. Pears is the right reference if Myrhiza is
willing to put determinism in the app's hands and use the substrate only for
event-log replication. Myrhiza's `state-apply` profile (per CLAUDE.md)
should look more Agoric-shaped than Pears-shaped.

## vs CRDTs (Automerge, Yjs) — the merge primitive

Pears' Autobase is CRDT-shaped without using CRDT terminology. Worth
flagging because the comparison is constantly mis-stated.

| Dimension | Pears (Autobase) | Automerge / Yjs |
|---|---|---|
| Convergence guarantee | "Linearizable view" — every replica derives same view from same set of operations | Strong eventual consistency — replicas converge regardless of arrival order |
| Conflict resolution | App-defined view function (deterministic given inputs) | Built-in per CRDT type (last-writer-wins, RGA for sequences, etc.) |
| Wire format | Hypercore blocks; merge happens at view-derivation time | CRDT operation log with vector clocks / Lamport timestamps |
| Compaction | Hypercore truncate + new-key-rotate | Yjs garbage collection, Automerge history compaction |
| Use case | App-defined semantics over an event log | App with off-the-shelf CRDT data structures |

| Dimension | Pears (Autobase) | Automerge / Yjs |
|---|---|---|
| Substrate | JS on Bare | JS / WASM library, embeddable anywhere |
| Mobile shipping | Yes (via Keet) | Yes (via Apple Notes-style apps using Automerge in 2024–2026) |
| Governance | Single vendor | Open-source community (Ink & Switch / Yjs community) |
| Maturity | Production via Keet | Production via several Ink & Switch backed apps |

**The verdict.** Autobase is the right primitive when your event semantics
are app-specific. Automerge / Yjs is the right primitive when your data
shape fits one of their built-in CRDT types. Myrhiza's `state-apply` is
closer to Autobase: app defines the merge.

## Summary — When to Borrow What

| Need | Reference |
|---|---|
| Production mobile P2P shipping evidence | **Pears** (Keet) |
| Embeddable Rust transport for WASM hosts | **Iroh** |
| Substrate-enforced determinism | **Agoric SwingSet** |
| Capability-typed RPC | **Spritely OCapN** |
| Multi-vendor governed substrate | **WASM CM / Bytecode Alliance** |
| Off-the-shelf CRDT data types | **Automerge / Yjs** |
| Agent-centric multi-writer with validation rules | **Holochain** |
| Append-only log + app-defined merge | **Pears (Autobase) / Agoric** |

Myrhiza will likely end up *triangulating*: WASM-CM substrate (for
determinism), Iroh-shaped transport (for embedding), Hypercore-shaped data
layer (for the proven-mobile-shipping append-only-log pattern), OCapN-shaped
RPC (for capabilities). Pears' role in that mix is *operational evidence*,
not architectural import.

## Cross-references

- [governance.md](./governance.md) — single-vendor risk relative to alternatives
- [lessons.md](./lessons.md) — concrete validates / avoid / borrow tables
- [Iroh](../iroh/) | [Holochain](../holochain/) | [WASM CM](../wasm-component-model/)
- [wasmCloud](../wasmcloud/) | [Spritely OCapN](../spritely-ocapn/) | [Agoric](../agoric-endo/)
