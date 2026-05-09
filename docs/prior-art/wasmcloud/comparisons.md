**Date:** 2026-05-09
**Status:** active
**Subject:** wasmCloud — comparative analysis against neighbors Myrhiza cares about

# Comparisons

wasmCloud is the closest existing art to a "production CM runtime that brokers I/O via capabilities." It sits in a crowded neighborhood — Spin, Extism, raw Wasmtime, Holochain, Spritely, Agoric, Iroh — and Myrhiza borrows from several of them simultaneously. This file pins the differences on the dimensions Myrhiza's design actually cares about.

Each comparison is a two-row table: what each system does, then how they differ along the axis Myrhiza must pick.

## wasmCloud vs Spin (Fermyon)

Both are Wasmtime + Component Model. Both are well-funded, CNCF-adjacent (Spin is sandbox; wasmCloud is incubating). The contrast is the *unit of deployment*.

| Axis | wasmCloud | Spin |
|---|---|---|
| What it is | Distributed runtime: lattice of hosts, components addressed by interface, brokered by NATS (v1) / K8s (v2). | Single-process serverless runtime: HTTP request comes in, component runs, response goes out. |
| Component shape | Function-style; lifetime is per-invocation; no long-lived state. | Server-style; component is the HTTP/Redis/cron handler; state via wasi-keyvalue/sqlite components. |
| Operational model | Operator runs a fleet (`wash` v1; `kubectl apply` for v2). | `spin up` — local dev or `spin deploy` to Fermyon Cloud / OCI. |
| I/O brokering | Capability providers (v1) or K8s-managed plugins (v2) implement WIT interfaces; components import. | Built-in trigger types (HTTP, Redis, MQTT, cron) plus wasi-keyvalue/sqlite/llm components. |
| Maturity feel | More flexible / operationally rich; steeper onramp; v2 is a hard pivot. | More polished out-of-box; SDK ergonomics ahead; smaller surface. |
| Repo signal (2026-05-09) | wasmCloud/wasmCloud: 2,301 stars, v2.1.0 (2026-05-07). | spinframework/spin: 6,407 stars, v4.0.0 (2026-04-20). |

**Myrhiza takeaway:** Myrhiza's components are closer to Spin's *function-style* invocation than to wasmCloud's *long-lived server* model — but Myrhiza's deployment topology is closer to wasmCloud's *fleet* than to Spin's *per-request serverless*. We end up between the two: function-shaped components, peer-distributed runtime, no central scheduler.

## wasmCloud vs Extism

Different problem space. Surface comparison only.

| Axis | wasmCloud | Extism |
|---|---|---|
| What it is | Component runtime: operate a fleet, components are first-class workloads. | Plugin runtime: embed a WASM plugin into a host application written in any language. |
| Host model | wasmCloud *is* the host. | Application *embeds* Extism; the application is the host. |
| Capability model | WIT-typed imports satisfied by capability providers / K8s plugins. | Host-defined function imports (PDK-style); narrower, app-specific. |
| Component Model | Yes; CM is first-class as of v1. | Mostly core WASM modules + custom ABI; CM support exists but less central. |
| Repo signal | 2,301 stars. | extism/extism: 5,601 stars. |

**Myrhiza takeaway:** Myrhiza is a runtime, not a plugin host — closer to wasmCloud than Extism on this axis. But Extism's "the application owns the host" framing is interesting for Myrhiza's *interaction-profile* components (UIs that embed app logic). Worth keeping the Extism PDK shape in mind for that profile.

## wasmCloud vs raw Wasmtime + custom orchestration

The "what does wasmCloud add over rolling-your-own?" question.

| Axis | wasmCloud | Wasmtime + custom |
|---|---|---|
| What you get | Lattice abstraction; link definitions; NATS bus (v1) or K8s control plane (v2); capability provider system; OCI distribution; wadm declarative manifests (v1). | Whatever you build. The WASM execution is the easy part. |
| What you give up | Tight coupling to NATS (v1) or K8s (v2); CM ABI is forced (good); one-vendor stewardship. | All of the above is now your problem. |
| Best fit | You want to operate a fleet of CM components and the architecture wasmCloud picked is acceptable. | You have idiosyncratic ops requirements (P2P-symmetric, on-device, browser, embedded). |

**Myrhiza takeaway:** Myrhiza is the "raw Wasmtime + custom orchestration" path — *because* the off-the-shelf orchestration assumes a central broker (NATS) or central scheduler (K8s). Both are anti-patterns for peer-symmetric P2P. We pay for the lattice abstraction in code we write, but we get peer-symmetry in return.

## wasmCloud vs Holochain

Both flavor as "P2P-ish app runtimes." The architectures are not similar.

| Axis | wasmCloud | Holochain |
|---|---|---|
| Network shape | NATS-as-control-plane (v1): a federated message broker, not symmetric P2P. v2 swaps NATS for K8s — even less P2P. | Gossip + DHT, peer-symmetric, no central broker. |
| Compute unit | WASM component implementing one or more WIT interfaces. | "Zome" — Rust-compiled-to-WASM module exposing functions; one or more zomes per "DNA" (app). |
| Capability discipline | Operator-declared link definitions; component-to-component calls authorized by lattice config. | Per-zome capability tokens; agent-to-agent calls authorized by cap grant. |
| State model | None at the runtime layer; live-only execution. | Source-chain (per-agent append-only log) + DHT (validated CRDT-ish global state). |
| Identity | NATS account / K8s identity. | Per-agent Ed25519 key, sovereign. |
| Determinism | Not a runtime concern. | Validation functions are deterministic by spec; gossip integrity depends on it. |

**Myrhiza takeaway:** Holochain is the *closer* design point on the network axis (gossip, peer-symmetric, sovereign identity). wasmCloud is the closer design point on the *component model* axis (WIT-typed, CM-first, OCI-distributed). Myrhiza is roughly **wasmCloud's component model on top of Holochain's network shape** — with Iroh as the actual transport. See [`../holochain/lessons.md`](../holochain/lessons.md).

## wasmCloud vs Spritely Goblins / OCapN

The "what is capability-typed RPC actually" comparison.

| Axis | wasmCloud | Spritely Goblins / OCapN |
|---|---|---|
| RPC model | wRPC: interface-typed RPC over NATS (v1) or direct (v2). The *interface* is the type; the *target* is resolved by lattice config. | Object-capability RPC: the cap-reference *is* the unforgeable authority. Sturdy refs, sealers, vows, three-party handoff. |
| Authority source | Operator-declared link definitions. RBAC-flavored. | Cryptographic ocaps; held by the holder; no operator. |
| Composition | Components compose via link definitions written by humans. | Objects compose by passing references over the wire; no human in the loop. |
| Maturity / scope | Production runtime; thousands of components in known deployments. | Reference design + Goblins (Racket / Guile); OCapN is a spec-in-progress. |

**Myrhiza takeaway:** wasmCloud's authority model is *not* capability-discipline in the OCap sense — it's operator-grant. Myrhiza wants real capability-discipline (a cap is unforgeable, transferable, revocable, transparently delegated) — closer to OCapN. But Myrhiza needs WIT-typed interfaces underneath, like wasmCloud. The synthesis is **WIT-typed interfaces + cryptographic ocap authority** — a place neither wasmCloud nor Spritely sits exactly. See [`../spritely-ocapn/lessons.md`](../spritely-ocapn/lessons.md).

## wasmCloud vs Agoric SwingSet

Both are typed-component runtimes with capability mediation. Different determinism postures.

| Axis | wasmCloud | Agoric SwingSet |
|---|---|---|
| Runtime | Wasmtime; live-only; no replay. | xsnap (Moddable's heavily-modified XS engine for JS); deterministic snapshot + replay. |
| Component shape | WIT-typed CM components. | "Vats" — JS object graphs with E-language ocap discipline. |
| Determinism | Not enforced. | Strict; the entire vat is replayable from an event log. |
| Capability discipline | Operator-declared. | Hardened-JS ocaps; the language enforces. |
| Why-it-exists | Run server fleets of WASM workloads. | Run smart-contract-shaped logic with consensus underneath. |

**Myrhiza takeaway:** Agoric is the closest existing art for Myrhiza's `state-apply` profile — a runtime where determinism is a *correctness* property, not a nice-to-have. wasmCloud has nothing to say about determinism. The lesson: when Myrhiza's `state-apply` profile lands, look at SwingSet's snapshot / replay machinery as a reference, not at wasmCloud. See [`../agoric-endo/lessons.md`](../agoric-endo/lessons.md).

## wasmCloud vs Iroh

Not a peer comparison — a substrate comparison.

| Axis | wasmCloud | Iroh |
|---|---|---|
| Layer | Runtime (above the transport). | Transport (below the runtime). |
| Network model (wasmCloud v1) | NATS-as-bus: federated broker, hub-and-spoke, multi-tenant via lattice IDs. | QUIC + relay-with-direct-upgrade; peer-symmetric; per-key endpoints. |
| If you swap | Replace NATS with iroh-net → wasmCloud architecture survives, network model becomes peer-symmetric. *This is roughly Myrhiza's bet.* | N/A — Iroh is the substrate. |

**Myrhiza takeaway:** Iroh as the transport substrate, with a wasmCloud-shaped runtime on top, is essentially Myrhiza's architecture. Where wasmCloud wires NATS subjects, Myrhiza wires ALPN-multiplexed iroh streams. Where wasmCloud's lattice gossip is NATS-cluster federation, Myrhiza's is gossipsub over iroh. The runtime/transport split is a real seam — Myrhiza inherits the seam from wasmCloud and the substrate choice from Iroh. See [`../iroh/lessons.md`](../iroh/lessons.md).

## Where this lands

| If Myrhiza wants… | …copy from… | …avoid from… |
|---|---|---|
| WIT-typed interfaces as the runtime ABI | wasmCloud, Spin | — |
| Function-style invocation, no long-lived component state | Spin | wasmCloud (long-lived workloads) |
| Peer-symmetric network, sovereign identity | Holochain, Iroh | wasmCloud (NATS / K8s central) |
| Cryptographic capability discipline | Spritely OCapN, Agoric | wasmCloud (operator-grant RBAC) |
| Determinism in `state-apply` | Agoric SwingSet | wasmCloud (live-only) |
| OCI-as-component-registry | wasmCloud | — |
| Declarative reconciliation (manifests → reality) | wasmCloud (wadm v1), Kubernetes | — |
| Lattice / interface-based addressing | wasmCloud (wRPC) | — |

## See also

- [`architecture.md`](architecture.md) — wasmCloud's lattice / host / provider topology.
- [`capability-model.md`](capability-model.md) — link definitions, contract IDs, claims.
- [`wrpc.md`](wrpc.md) — wRPC: interface-typed RPC, the bus, the wire format.
- [`critiques.md`](critiques.md) — single-vendor risk, NATS dependency, v2 K8s pivot.
- [`lessons.md`](lessons.md) — validates / avoid / borrow tables.
- Prior-art neighbors: [Iroh](../iroh/), [Holochain](../holochain/), [Spritely OCapN](../spritely-ocapn/), [Agoric SwingSet](../agoric-endo/), [WASM Component Model](../wasm-component-model/).
