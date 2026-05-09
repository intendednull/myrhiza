# Lessons for Myrhiza

This is the consult-this-when-designing file. The other prior-art files are evidence; this file is decisions.

## Validates

These Myrhiza design choices are confirmed by Holochain's experience:

- **Peer-symmetric / no global consensus.** Holochain has shipped real apps without consensus since 2020. The agent-centric source-chain + per-op validation pattern works; it scales to thousands of nodes; the immune-system metaphor with warrants is actually deployable.
- **Deterministic-validation host fn split.** The integrity/coordinator separation is real and necessary. Myrhiza's deterministic state-apply boundary is the same idea expressed in WIT — keep the split.
- **Capability-mediated I/O.** Holochain's grants prove that agents-as-keys + per-call-authority works. It's also strictly weaker than typed handles, which validates Myrhiza's choice to use Component Model handles instead of secret-bearer tokens.
- **iroh as transport.** Holochain converged on iroh in the 0.6.1-rc line (early–mid 2026) after a multi-year detour through n3h → lib3h → sim1h → sim2h → kitsune1 → kitsune2 over WebRTC/tx5. Myrhiza starts on iroh. The detour is the cost worth quoting; don't reinvent it.
- **Lazy-loaded, hash-addressed components.** DNA hash = network identity is the same idea as content-addressed components. Validates the model.
- **Local-first UI over websocket.** UI talks to local conductor over capability-gated websocket. Myrhiza's planned UI bridge can borrow this shape.

## Avoid

| Pitfall | Source | Myrhiza mitigation |
|---|---|---|
| **Custom WASM ABI that can't survive a host upgrade.** HDK breakage every minor release is the symptom. | [`distribution.md`](distribution.md) | Lock onto WIT-typed interfaces from day one. Version interfaces explicitly. Allow multiple interface versions to coexist in one runtime. |
| **Ship a "WASM conductor / light client" as a roadmap item rather than a foundation.** Holochain has been promising browser conductors since 2019. | [`open-problems.md`](open-problems.md#8-light-clients--browser-native-runtime) | Browser viability via jco is a load-bearing requirement. Don't add it later. |
| **Bake identity into per-app keys with no canonical multi-device/rotation story.** DPKI shipped, then was deleted in 0.6 after 7 years of effort. | [`identity.md`](identity.md) | Decide early whether identity is a first-class runtime primitive or explicitly delegated to apps. Do not half-ship it. |
| **Centralized "switchboard" crutches in the dev path.** sim2h was a sandbox that became culturally entrenched and slowed the real P2P work. | [`networking.md`](networking.md) | Don't introduce a centralized signaler as a "temporary" dev shortcut without a hard sunset date. iroh gives you NAT traversal + relays as a real solution. |
| **Coarse-grained capabilities (per-zome-function, secret-bearer).** Granularity is too low and bearer tokens leak. | [`capabilities.md`](capabilities.md) | Use Component Model resource handles. Capabilities should be transferable values with non-forgeable types. |
| **Treating sharding as "we'll finish it later."** Six+ years on, partial arcs are not the load-tested path. | [`networking.md`](networking.md), [`open-problems.md`](open-problems.md) | Decide what sharding model you're committing to and load-test it from MVP. Or commit to "every node holds everything" with explicit scale ceiling. |
| **Tight coupling of integrity definition to network identity (DNA hash).** Means any data-model bugfix forks the network. | [`architecture.md`](architecture.md), [`distribution.md`](distribution.md) | Allow integrity changes to migrate forward without forced-fork. Schema evolution is an explicit product surface. |
| **Sybil/free-rider hand-waving.** "We don't have consensus so we don't have these problems" is not true; the problems just relocate. | [`open-problems.md`](open-problems.md) | Ship explicit guidance for app authors on membrane proofs, stake, or proof-of-personhood. Don't pretend the runtime solves it. |
| **"No built-in concept of group."** Has caused every app to invent its own group abstraction badly. | [`open-problems.md`](open-problems.md) | Decide whether group-of-agents is a runtime primitive or a documented pattern. Don't accidentally avoid the choice. |
| **Manifest/conductor-config thrash every release.** | [`distribution.md`](distribution.md) | Stabilize the manifest format earlier than feels comfortable. Treat it like a public API. |
| **Per-app reinvention of group cryptography.** Every messaging hApp rolls its own key derivation. | [`identity.md`](identity.md) | Make MLS (or equivalent) a host import, not an app-level reinvention. |
| **Builder-tools-for-builders trap.** Holochain has framework users (hREA, Neighbourhoods) but no flagship consumer app after 8+ years. | [`apps.md`](apps.md) | Make sure Myrhiza's own demos target end users, not other framework developers. |

## Borrow

Concrete subsystems worth deep study (and possibly direct adaptation):

1. **Source chain.** Per-agent, hash-linked, signed, append-only log of all locally-authored actions. This is the right primitive for "what did I do and can I prove it." Steal the data structure wholesale. See [`architecture.md`](architecture.md).
2. **DHT op decomposition.** A single commit produces multiple op types (`StoreEntry`, `StoreRecord`, `RegisterAgentActivity`, `RegisterUpdate`, `RegisterDelete`, link ops) each routed to different basis hashes. Lets different authority sets validate different facets of the same action. Generalize to: "one logical write fans into N op types, each with its own authority basis." See [`architecture.md`](architecture.md).
3. **Warrants.** Signed proofs of misbehavior, gossip-distributed, used as block-list evidence. Cleaner than a reputation score; harder to game. See [`identity.md`](identity.md).
4. **Countersigning protocol.** Explicit framework support for "two+ agents commit the same entry to all their chains, atomically, within a time window." Includes "enzyme" pattern for an asymmetric coordinator. Steal this as Myrhiza's atomic-multi-party primitive. See [`determinism.md`](determinism.md).
5. **Validation as a pure WASM callback in a restricted host-fn subset.** Deterministic by construction. Myrhiza has the same need; the integrity/coordinator split with two HDK subsets is exactly the shape — ours is enforced statically via Component Model imports. See [`determinism.md`](determinism.md).
6. **Membrane proofs.** App-defined join proofs ("you may enter this DHT iff you can present X"). Pluggable Sybil gating without baking a specific scheme into core. See [`open-problems.md`](open-problems.md).
7. **`must_get_*` with unresolved-dependency retry.** Validation declares "I need op X to decide." If X isn't local, the op parks in a queue and re-validates on arrival. Avoids partial-knowledge invalid-validations. Lift this pattern. See [`determinism.md`](determinism.md).
8. **The Kitsune2 gossip cadence (1 min for newcomers, back off to 5 min).** Empirically tuned. Don't reinvent — copy the curve. See [`networking.md`](networking.md).
9. **hApp manifest as a graph of role → DNA → zome.** The role layer (one DNA can be installed under multiple symbolic roles) is genuinely useful. Component Model has analogous wiring — borrow the role concept as your composition primitive. See [`distribution.md`](distribution.md).
10. **CAL-1.0 license model.** Worth studying for how to encode user-data/key rights in the license itself, not just the project ethos. See [`governance.md`](governance.md#licensing--cal-10).

## How to use this file

When designing a Myrhiza feature:

1. Find the row in **Avoid** that names a pitfall close to your design. Read the linked subsystem file for the full incident.
2. Find the row in **Borrow** that names a primitive close to what you're designing. Read the upstream Holochain docs to understand the shape, then adapt for Component Model.
3. Promote any decision into a Myrhiza spec under `docs/specs/` — this file is for capturing what we learn, not for encoding our decisions.
