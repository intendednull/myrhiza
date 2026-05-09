# Open problems Holochain hasn't solved

These are problems the architecture doesn't structurally solve, regardless of effort applied. Myrhiza will face most of them — be modest about the same boundaries.

## 1. Sybil resistance

Generating new agent keys is free. Joining proofs (membrane proofs) gate entry to a specific DHT but don't prevent identity-multiplication within a permissive DHT. ([Sybil discussion](https://www.tandfonline.com/doi/full/10.1080/17445760.2024.2352740)).

No proof-of-personhood. No stake. The "agent-centric" framing doesn't dissolve the problem; it relocates it from "who can write to the chain" to "who can join the network."

## 2. Free-rider participation

Nodes can declare zero-size storage arcs and freely consume the DHT without contributing storage. Enforcement is per-app and ad hoc.

The "Sybil-resistance / permissionlessness / free-ness trilemma" is real: you can have any two but not all three.

## 3. Distributed maintenance / data resilience

If a neighborhood depopulates (everyone with arc covering range X goes offline), the data in that range becomes unreachable. Erasure-coding / replication-factor SLAs are not a first-class primitive.

Apps that need durability guarantees end up running their own pinning workers, which is exactly the architecture Myrhiza's PR research notes flag as the big open problem.

## 4. Large-entry gossip

The DHT is optimized for many small entries, not for blob storage. Apps that need media storage typically build a separate pinning layer.

## 5. Sharding completion

Partial storage arcs work but aren't the canonical path; full arcs are still the load-tested config. After 6+ years.

This is the canonical case study for "we'll finish sharding later" — the easy thing (full arcs) becomes the load-tested path, the hard thing (partial arcs under arbitrary topology) never converges.

## 6. Group cryptography

No MLS, no shared group-key story. Every messaging hApp re-implements forward-secret group messaging — and most do it badly.

A runtime-level primitive would have prevented years of duplicated effort.

## 7. Cross-DNA discovery and group identity

"There is no built-in concept of a group" ([Basis critique](https://basisproject.net/posts/2020/04/valueflows-blockchain-holochain/)). DNAs are the unit; agent identity is per-DNA; cross-DNA composition is by app-level convention.

Apps that span multiple DNAs (e.g. a chat app that wants to bridge to a project-management app) have to invent ad-hoc cross-DNA wiring. The runtime doesn't help.

## 8. Light clients / browser-native runtime

Roadmap item since 2019 ([WASM Conductor and Light Client groundwork](https://blog.holochain.org/the-groundwork-for-the-wasm-conductor-and-light-client/)), still not shipped. The conductor is a Rust process; UIs are HTML+JS connecting via WebSocket from a Tauri-Mobile or Electron desktop wrapper using [`@holochain/client`](https://github.com/holochain/holochain-client-js). The standard distribution is the Holochain Launcher (Tauri-based) or Kangaroo / [p2p Shipyard](https://blog.holochain.org/happs-spotlight-relay/) wrapping per-hApp.

A full conductor wants long-running native sockets, persistent storage, libsodium, an out-of-process keystore — the browser has none natively, and a WASM port would have to replace each. Each replacement is its own subproject. Holo's pivot to an HTTP "Web Bridge" (Q1 2025) sidesteps the problem: web users hit a hosted node over HTTP rather than running a conductor.

**Why this matters for Myrhiza.** Holochain's runtime predates the Component Model. It compiles guest WASM with a Holochain-specific ABI (`hdk` macros + JSON-bincode wire format). That ABI was not designed for browser embedding and was not designed for guest-language pluralism. Myrhiza's bet on Component Model + jco lets you ship the same components to a native iroh runtime and to a browser jco-compiled JS shim **without re-architecting**. Holochain is bolting that on after 6+ years; you can have it as a first-class invariant.

**Lesson:** browser viability is a load-bearing requirement, not a roadmap item. Once an ABI exists with N apps depending on it, retrofitting browser viability becomes a multi-year project that competes with every other improvement.

## 9. Identity / key rotation

DPKI removed in 0.6; replacement TBD. See [`identity.md`](identity.md).

## 10. No formal verification

Holochain does **no formal verification or model-checking of its core algorithms**. Searches across the [holochain monorepo](https://github.com/holochain/holochain) and [lair](https://github.com/holochain/lair) turn up zero references to TLA+, Coq, Isabelle/HOL, Lean, [Kani](https://github.com/model-checking/kani), or [Loom](https://github.com/tokio-rs/loom). No specifications outside the implementation, no machine-checked proofs of validation correctness, gossip convergence, countersigning atomicity, or warrant propagation. The "strong eventual consistency" claim in [`concepts/7_validation`](https://developer.holochain.org/concepts/7_validation/) is asserted, not proven.

Correctness assurance is **entirely empirical**:

- **Sweettest** ([crates/sweettest](https://github.com/holochain/holochain/tree/develop/crates/sweettest)) — Rust integration harness that spins up multi-conductor topologies and asserts behavior. Standard unit/integration testing.
- **[Wind Tunnel](https://github.com/holochain/wind-tunnel)** — distributed performance and load-testing framework, 23 scenarios as of 2025. Wind Tunnel measures performance, *not* correctness.
- **[Least Authority audit](https://leastauthority.com/blog/audit-of-holochain-lair-keystore/)** of lair-keystore — manual code review, no formal artifacts.

Adjacent academic work exists but has not been applied to Holochain. The 2017 OOPSLA paper [Verifying Strong Eventual Consistency in Distributed Systems](https://arxiv.org/abs/1707.01747) (Gomes et al., Isabelle/HOL framework for CRDT convergence proofs) is the closest match for the kind of guarantee Holochain claims; nothing analogous has been done for the Holochain validation/gossip stack.

What it would take to add it:

- A **TLA+ specification** of the gossip + validation + warrant state machine — feasible scope for a dedicated formalist, weeks not years.
- **Loom-based concurrency tests** of the conductor's workflow triggers and the lair IPC handshake — these are pure-Rust, single-process subsystems and would benefit immediately. (The conductor's own comment in `update_coordinators` — *"this isn't really concurrent safe"* — is the kind of statement Loom would either confirm or refute.)
- A **Kani model** of the source-chain commit pipeline up to the wasmer boundary, proving no-double-commit and prev-action-link invariants under arbitrary scheduling.

None of this is on a public roadmap. For Myrhiza, this is a gap to *not* repeat: the core state machines (state-apply ordering, component link integrity, capability-token check) are bounded enough to specify in TLA+ from day one, and Loom is essentially free to adopt for any Rust runtime.

## Implications for Myrhiza

Don't pretend Myrhiza solves these. The PR's research-notes file already flags distributed maintenance as the biggest open problem, which is the right framing.

Specific decisions where this matters:

- **Membrane proofs (1, 2):** Borrow the primitive — pluggable join-gating where the runtime doesn't bake in a specific scheme but exposes the hook. Apps choose stake / proof-of-personhood / invite-graph / whatever.
- **Storage SLAs (3, 5):** Decide the model up front. "Every node holds everything for its app" with explicit scale ceiling, OR a sharding model that's load-tested from MVP. Don't ship "we'll figure it out."
- **Group cryptography (6):** Make MLS (or equivalent) a host import, not an app-level reinvention.
- **Group identity (7):** Decide whether group-of-agents is a runtime primitive or a documented pattern. Don't accidentally avoid the choice.
- **Sybil/free-rider (1, 2):** Document explicitly that the runtime doesn't solve these. Apps must choose how to gate; the runtime provides the mechanism, not the policy.

## Sources

- [Sybil attack vulnerability trilemma (Tandfonline)](https://www.tandfonline.com/doi/full/10.1080/17445760.2024.2352740)
- [Holochain for IoT — DLT review (PMC)](https://pmc.ncbi.nlm.nih.gov/articles/PMC12251913/)
- [Basis: ValueFlows, Holochain, and blockchain (group critique)](https://basisproject.net/posts/2020/04/valueflows-blockchain-holochain/)
- [2025 at a Glance: Landing Reliability](https://blog.holochain.org/2025-at-a-glance-landing-reliability/)
