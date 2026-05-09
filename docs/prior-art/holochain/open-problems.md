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

Roadmap item since 2019, still not shipped. See [`browser.md`](browser.md).

## 9. Identity / key rotation

DPKI removed in 0.6; replacement TBD. See [`identity.md`](identity.md).

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
